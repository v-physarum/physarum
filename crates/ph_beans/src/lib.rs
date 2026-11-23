//! Query pipeline for beans.
//!
//! One [`Descriptor`] carries pipeline status. A linear [`Ctx`] holds the
//! bean, the resource args, that status, and the systems. [`Setup::setup`]
//! returns [`Tick`] only when the status is [`PipelineOk`].

#![forbid(unsafe_code)]

extern crate self as ph_beans;

pub mod bean;
pub mod borrow;
pub mod context;
pub mod descriptor;
pub mod filter;
pub mod query;

pub use bean::{
    ApplyProxy, BeanInfo, EventInfo, FreeKey, FromParent, Fwd, InitPath, Marked, Module, More,
    OfInject, OfInject0, OfInject1, OfParent, OfProvide, OfProvide0, OfProvide1, ParentList,
    Proxied, Proxy, Stop, Target, Via,
};
pub use borrow::{At, Ix0, Ix1, Ix2, Ix3, Off, Take, TakeMany};
pub use context::{
    Context, ContextTick, Ctx, Leaf, List, MAX_MAP_SYSTEMS, MapList, NoSystems, Pipe, StartDesc,
    WithDefault, WithInit,
};
pub use descriptor::{
    Accessors, Descriptor, Dyn, DynView, ErrBadEvent, ErrConflictAccess, ErrDuplicateResource,
    ErrInitAlreadySet, ErrNeedWaitFilter, ErrTooManySystems, Get, Idle, Live, Marker, Mut,
    NeedBuild, NeedInit, OpDesc, OpOk, OpShape, PipelineOk, Valid, WarnUnusedArgs,
};
pub use filter::{And, ApplyFilter, NestOk, Not, On, Or};
pub use query::{
    BeanTag, Built, ClaimVia, FromChain, FromProxy, Init, Own, Query, Setup, Surface, Tick, Update,
};

#[cfg(test)]
mod tests {
    mod proxy {
        //! `Id` is the manual plain bean. `Row0` through `Row6` repeat that impl.
        //!
        //! `emitting_the_proxy_runs_the_target` is the manual emit.
        //! `emitting_chain_level_*` repeats it for the chain.
        //! `the_target_sees_the_proxy_provides` injects `Score` from `Update`'s `Provides`.
        //! `GameTick`: `Proxy = Update`, so `Link = Get<Update>`.

        use std::fs;
        use std::path::PathBuf;
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        use core::cell::Cell;

        use ph_test_errors::ErrorTester;

        use ph_beans::{
            At, BeanInfo, Ctx, EventInfo, Fwd, Get, Init, InitPath, Ix0, Ix1, Live, Marker, More,
            Mut, Pipe, Proxied, Proxy, Query, Stop, Target, Tick, Via, WithInit,
        };
        use ph_util_foreach::impl_foreach;

        thread_local! {
            static HITS: Cell<u8> = const { Cell::new(0) };
            static ORDER: Cell<u16> = const { Cell::new(0) };
        }

        fn idle(_: ()) {}

        fn core_state(_: ()) -> u8 {
            0
        }

        fn hit() {
            HITS.with(|hits| hits.set(hits.get() + 1));
        }

        fn reset() {
            HITS.with(|hits| hits.set(0));
        }

        fn mark(step: u16) {
            ORDER.with(|order| order.set(order.get() * 10 + step));
        }

        fn reset_order() {
            ORDER.with(|order| order.set(0));
        }

        struct Id;

        impl BeanInfo for Id {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Id>;

            type Link = ph_beans::Get<Id>;
            type Chain = ph_beans::Stop;
        }
        impl Target for Id {}

        impl_foreach!(n in 0..7 {
            struct paste(Row, n);
            impl BeanInfo for paste(Row, n) {
                type Provides = ();
                type Injects = ();
                type Beans = ();
                type State = ();

                type Proxy = Proxy<paste(Row, n)>;
                type Link = Get<paste(Row, n)>;
                type Chain = Stop;
            }
            impl Target for paste(Row, n) {}
        });
        type Score = Row0;
        type Main = Row1;
        type Host = Row2;
        type Plain = Row3;
        type Prize = Row4;
        type Bonus = Row5;
        type Alarm = Row6;

        impl EventInfo for Main {
            type Parents = ();
        }
        impl EventInfo for Plain {
            type Parents = ();
        }
        impl EventInfo for Alarm {
            type Parents = ();
        }

        struct Shell;
        struct Update;
        struct GameTick;
        struct Gate;
        struct Loud;

        impl BeanInfo for Shell {
            type Provides = (Id, GameTick);
            type Injects = ();
            type Beans = (Id, GameTick);
            type State = ();

            type Proxy = Proxy<Shell>;

            type Link = ph_beans::Get<Shell>;
            type Chain = ph_beans::Stop;
        }
        impl Target for Shell {}
        impl At<Id> for Shell {
            type Ix = Ix0;
        }
        impl At<GameTick> for Shell {
            type Ix = Ix1;
        }

        impl BeanInfo for Update {
            type Provides = (Score,);
            type Injects = ();
            type Beans = (Score,);
            type State = ();

            type Proxy = Proxy<Update>;

            type Link = Get<Update>;
            type Chain = Stop;
        }
        impl At<Score> for Update {
            type Ix = Ix0;
        }
        impl EventInfo for Update {
            type Parents = Main;
        }
        impl InitPath<GameTick> for Update {
            fn default_state() -> u8 {
                7
            }
        }

        impl BeanInfo for GameTick {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = u8;

            type Proxy = Update;

            type Link = ph_beans::Get<GameTick>;
            type Chain = ph_beans::Stop;
        }
        impl Target for GameTick {}
        impl EventInfo for GameTick {
            type Parents = ();
        }

        impl BeanInfo for Gate {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Gate>;

            type Link = Mut<Gate>;
            type Chain = Stop;
        }

        impl BeanInfo for Loud {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Gate;

            type Link = ph_beans::Get<Loud>;
            type Chain = ph_beans::Stop;
        }
        impl Target for Loud {}

        struct Core;
        struct Inner;
        struct Mid;
        struct Outer;
        struct Room;

        impl BeanInfo for Core {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = u8;

            type Proxy = Inner;

            type Link = ph_beans::Get<Core>;
            type Chain = ph_beans::Stop;
        }
        impl Target for Core {}
        impl EventInfo for Core {
            type Parents = ();
        }

        impl BeanInfo for Inner {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Inner>;

            type Link = Get<Inner>;
            type Chain = More<Mid>;
        }
        impl EventInfo for Inner {
            type Parents = ();
        }
        impl InitPath<Core> for Inner {
            fn default_state() -> u8 {
                7
            }

            fn enter() {
                mark(3);
            }
        }

        impl BeanInfo for Mid {
            type Provides = (Prize,);
            type Injects = ();
            type Beans = (Prize,);
            type State = ();

            type Proxy = Proxy<Mid>;

            type Link = Get<Mid>;
            type Chain = More<Outer>;
        }
        impl EventInfo for Mid {
            type Parents = ();
        }
        impl At<Prize> for Mid {
            type Ix = Ix0;
        }
        impl InitPath<Inner> for Mid {
            fn enter() {
                mark(2);
            }
        }

        impl BeanInfo for Outer {
            type Provides = (Bonus,);
            type Injects = ();
            type Beans = (Bonus,);
            type State = ();

            type Proxy = Proxy<Outer>;

            type Link = Get<Outer>;
            type Chain = Stop;
        }
        impl EventInfo for Outer {
            type Parents = Alarm;
        }
        impl At<Bonus> for Outer {
            type Ix = Ix0;
        }
        impl InitPath<Mid> for Outer {
            fn enter() {
                mark(1);
            }
        }

        impl BeanInfo for Room {
            type Provides = (Core,);
            type Injects = ();
            type Beans = (Core,);
            type State = ();

            type Proxy = Proxy<Room>;

            type Link = ph_beans::Get<Room>;
            type Chain = ph_beans::Stop;
        }
        impl Target for Room {}
        impl At<Core> for Room {
            type Ix = Ix0;
        }

        trait Same<T> {}
        impl<T> Same<T> for T {}

        trait Through<P> {}
        impl<H, P, L> Through<P> for Pipe<WithInit<Fwd<P, H>>, L> {}

        type DefaultWrap<S, T> = Init<(), Ctx<Shell, Live<Via<Proxy<Id>, Get<Id>>>, S, T>>;
        type UpdateWrap<S, T> = Init<(), Ctx<Shell, Live<Via<Update, Get<GameTick>>>, S, T>>;
        type ChainClaim = Via<Outer, Via<Mid, Via<Inner, Get<Core>>>>;
        type ChainWrap<S, T> = Init<(), Ctx<Room, Live<ChainClaim>, S, T>>;
        type ChainFwd<H> = Fwd<Outer, Fwd<Mid, Fwd<Inner, H>>>;

        fn assert_default<S, T>(_: &DefaultWrap<S, T>) {}

        fn assert_explicit<S, T>(_: &UpdateWrap<S, T>) {}

        fn assert_init<E, A, S, T, P>(_: &Init<E, Ctx<Host, A, S, T>>)
        where
            T: Through<P>,
        {
        }

        fn assert_chain<S, T>(_: &ChainWrap<S, T>) {}

        trait ChainThrough {}

        impl<H, L> ChainThrough for Pipe<WithInit<ChainFwd<H>>, L> {}

        fn assert_chain_init<E, A, S, T>(_: &Init<E, Ctx<Core, A, S, T>>)
        where
            T: ChainThrough,
        {
        }

        #[test]
        fn an_unset_proxy_uses_the_default() {
            let pipeline = Shell::query().inject_resources(Id::T);
            assert_default(&pipeline);
        }

        #[test]
        fn injecting_the_target_wraps_the_explicit_proxy() {
            let pipeline = Shell::query().inject_resources(GameTick::T);
            assert_explicit(&pipeline);
        }

        #[test]
        fn link_records_shared_and_exclusive_markers() {
            fn shared<T: Same<Get<Update>>>() {}
            fn exclusive<T: Same<Mut<Gate>>>() {}
            fn default_link<B>()
            where
                B: BeanInfo<Proxy = Proxy<B>> + Proxied<Link = Get<Proxy<B>>>,
            {
            }
            fn named<P: Same<Update>>() {}
            fn gate<P: Same<Gate>>() {}
            fn inner<P: Same<Inner>>() {}
            default_link::<Id>();
            shared::<<GameTick as Proxied>::Link>();
            exclusive::<<Loud as Proxied>::Link>();
            named::<<GameTick as BeanInfo>::Proxy>();
            gate::<<Loud as BeanInfo>::Proxy>();
            inner::<<Core as BeanInfo>::Proxy>();
            shared::<<Update as BeanInfo>::Link>();
            exclusive::<<Gate as BeanInfo>::Link>();
        }

        #[test]
        fn the_target_sees_the_proxy_provides() {
            let _surface = GameTick::query().inject_resources(Score::T);
        }

        #[test]
        fn a_bean_listed_on_the_module_and_the_proxy_is_rejected() {
            // Metadata-only probes skip the const-eval that reports this. A full
            // rustc build is what prints it. The module lists Score, and the
            // proxy lists Score in Provides.
            let source = "
                use ph_beans::{At, BeanInfo, BeanTag, Get, Ix0, Marker, Proxy, Query, Stop, Target};
                struct Score;
                impl BeanTag for Score { const ID: u64 = 1; }
                struct Update;
                struct GameTick;
                impl BeanInfo for Score {
                    type Provides = (); type Injects = (); type Beans = (); type State = ();
                    type Proxy = Proxy<Score>;
                    type Link = ph_beans::Get<Score>;
                    type Chain = Stop;
                }
                impl Target for Score {}
                impl BeanInfo for Update {
                    type Provides = (Score,); type Injects = (); type Beans = (Score,); type State = ();
                    type Proxy = Proxy<Update>;
                    type Link = Get<Update>;
                    type Chain = Stop;
                }
                impl At<Score> for Update { type Ix = Ix0; }
                impl BeanInfo for GameTick {
                    type Provides = (); type Injects = (); type Beans = (Score,); type State = ();
                    type Proxy = Update;
                    type Link = ph_beans::Get<GameTick>;
                    type Chain = Stop;
                }
                impl Target for GameTick {}
                pub fn probe() { let _claim = GameTick::query().inject_resources(Score::T); }
                ";
            assert_full_rustc(source, "already on the proxy");
        }

        #[test]
        fn a_proxy_object_absent_from_provides_is_rejected() {
            fail(
                "
                use ph_beans::{At, BeanInfo, Get, Ix0, Marker, Proxied, Proxy, Query, Stop, Target};
                struct Score;
                struct Update;
                struct GameTick;
                impl BeanInfo for Score {
                    type Provides = (); type Injects = (); type Beans = (); type State = ();
                
                type Proxy = Proxy<Score>;
                type Link = ph_beans::Get<Score>;
                type Chain = ph_beans::Stop;
}
                impl Target for Score {}
                impl BeanInfo for Update {
                    type Provides = (); type Injects = (); type Beans = (); type State = ();
                
                type Proxy = Proxy<Update>;
                type Link = Get<Update>;
                type Chain = Stop;
}
                impl At<Score> for Update { type Ix = Ix0; }
                impl BeanInfo for GameTick {
                    type Provides = (); type Injects = (); type Beans = (); type State = ();
                
                type Proxy = Update;
                type Link = ph_beans::Get<GameTick>;
                type Chain = ph_beans::Stop;
}
                impl Target for GameTick {}
                fn probe() { let _claim = GameTick::query().inject_resources(Score::T); }
                ",
                "not on the proxy list",
            );
        }

        #[test]
        fn parents_on_the_target_stay_empty() {
            fn parents<E: EventInfo<Parents = ()>>() {}
            parents::<GameTick>();
        }

        #[test]
        fn a_proxy_parent_allows_the_target_under_that_event() {
            let _pipeline = Host::query().filter(Main::ON[GameTick::ON]);
        }

        #[test]
        fn emitting_the_proxy_runs_the_target() {
            reset();
            let mut tick = Host::query()
                .filter(GameTick::ON)
                .map(hit)
                .emit(Update::ON)
                .init_state(idle)
                .build();
            Tick::<Host>::update(&mut tick, ());
            assert_eq!(HITS.with(Cell::get), 1);
        }

        #[test]
        fn emitting_the_proxy_parent_runs_the_target() {
            reset();
            let mut tick = Host::query()
                .filter(GameTick::ON)
                .map(hit)
                .emit(Main::ON)
                .init_state(idle)
                .build();
            Tick::<Host>::update(&mut tick, ());
            assert_eq!(HITS.with(Cell::get), 1);
        }

        #[test]
        fn a_default_proxy_does_not_inherit_foreign_events() {
            reset();
            let mut tick = Host::query()
                .filter(Plain::ON)
                .map(hit)
                .emit(Main::ON)
                .init_state(idle)
                .build();
            Tick::<Host>::update(&mut tick, ());
            assert_eq!(HITS.with(Cell::get), 0);
        }

        #[test]
        fn a_default_proxy_still_runs_its_own_event() {
            reset();
            let mut tick = Host::query()
                .filter(Plain::ON)
                .map(hit)
                .emit(Plain::ON)
                .init_state(idle)
                .build();
            Tick::<Host>::update(&mut tick, ());
            assert_eq!(HITS.with(Cell::get), 1);
        }

        #[test]
        fn init_of_an_unset_proxy_goes_through_the_default() {
            let pipeline = Host::query().init_state(idle);
            assert_init::<_, _, _, _, Proxy<Host>>(&pipeline);
        }

        #[test]
        fn init_default_comes_from_the_explicit_proxy() {
            let built = GameTick::query().init_state_default().build();
            assert_eq!(*built.state(), 7);
        }

        #[test]
        fn a_chain_nests_wrappers_outer_around_inner() {
            let pipeline = Room::query().inject_resources(Core::T);
            assert_chain(&pipeline);
        }

        #[test]
        fn the_bean_sees_provides_on_every_proxy_in_the_chain() {
            let _mid = Core::query().inject_resources(Prize::T);
            let _outer = Core::query().inject_resources(Bonus::T);
        }

        #[test]
        fn parents_on_a_chained_bean_stay_empty() {
            fn parents<E: EventInfo<Parents = ()>>() {}
            parents::<Core>();
        }

        #[test]
        fn an_outer_proxy_parent_allows_the_bean_under_that_event() {
            let _pipeline = Room::query().filter(Alarm::ON[Core::ON]);
        }

        type ChainEvent0 = Inner;
        type ChainEvent1 = Mid;
        type ChainEvent2 = Outer;

        impl_foreach!(n in 0..3 {
            #[test]
            fn paste(emitting_chain_level_, n)() {
                reset();
                let mut tick = Room::query()
                    .filter(Core::ON)
                    .map(hit)
                    .emit(paste(ChainEvent, n)::ON)
                    .init_state(idle)
                    .build();
                Tick::<Room>::update(&mut tick, ());
                assert_eq!(HITS.with(Cell::get), 1);
            }
        });

        #[test]
        fn emitting_an_outer_proxy_parent_runs_the_bean() {
            reset();
            let mut tick = Room::query()
                .filter(Core::ON)
                .map(hit)
                .emit(Alarm::ON)
                .init_state(idle)
                .build();
            Tick::<Room>::update(&mut tick, ());
            assert_eq!(HITS.with(Cell::get), 1);
        }

        #[test]
        fn init_of_a_chain_runs_outer_then_inner() {
            reset_order();
            let pipeline = Core::query().init_state(core_state);
            assert_chain_init(&pipeline);
            let built = Core::query().init_state_default().build();
            assert_eq!(ORDER.with(Cell::get), 123);
            assert_eq!(*built.state(), 7);
        }

        #[test]
        fn injecting_a_proxy_is_rejected() {
            fail(
                "
            use ph_beans::{At, BeanInfo, Ix0, Marker, Proxy, Query, Stop, Target};
            struct Tick;
            struct Update;
            struct App;
            impl BeanInfo for Tick {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Tick>;
            type Link = ph_beans::Get<Tick>;
            type Chain = ph_beans::Stop;
}
            impl Target for Tick {}
            impl BeanInfo for Update {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Update>;
            type Link = ph_beans::Get<Update>;
            type Chain = Stop;
}
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl At<Update> for App { type Ix = Ix0; }
            fn probe() { let _claim = App::query().inject_resources(Update::MUT); }
            ",
                "not an injectable bean",
            );
        }

        #[test]
        fn injecting_a_proxy_provide_is_rejected() {
            fail(
            "
            use ph_beans::{BeanInfo, FromParent, Get, OfProvide, Proxy, Stop, Target};
            struct Tick;
            struct Update;
            struct App;
            impl BeanInfo for Tick {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Tick>;
            type Link = ph_beans::Get<Tick>;
            type Chain = ph_beans::Stop;
}
            impl Target for Tick {}
            impl BeanInfo for Update {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Update>;
            type Link = Get<Update>;
            type Chain = Stop;
}
            impl BeanInfo for App {
                type Provides = Get<Update>; type Injects = (); type Beans = Get<Update>; type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            fn probe() { fn inject<C: FromParent<App, OfProvide>>() {} inject::<Get<Update>>(); }
            ",
            "not an injectable bean",
        );
        }

        #[test]
        fn claiming_a_proxy_is_rejected() {
            fail(
                "
            use ph_beans::query::Claim;
            use ph_beans::{At, BeanInfo, Ix0, Proxy, Stop, Target};
            struct Tick;
            struct Update;
            struct App;
            impl BeanInfo for Tick {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Tick>;
            type Link = ph_beans::Get<Tick>;
            type Chain = ph_beans::Stop;
}
            impl Target for Tick {}
            impl BeanInfo for Update {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Update>;
            type Link = ph_beans::Get<Update>;
            type Chain = Stop;
}
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl At<Update> for App { type Ix = Ix0; }
            fn probe<C: Claim<App>>() {}
            fn call() { probe::<ph_beans::Mut<Update>>(); }
            ",
                "not an injectable bean",
            );
        }

        #[test]
        fn injecting_an_inner_proxy_is_rejected() {
            fail(
                "
            use ph_beans::{At, BeanInfo, Get, Ix0, Marker, More, Proxied, Proxy, Query, Stop};
            struct Tick;
            struct Inner;
            struct Outer;
            struct App;
            impl BeanInfo for Tick {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Inner;
            type Link = ph_beans::Get<Tick>;
            type Chain = ph_beans::Stop;
}
            impl ph_beans::Target for Tick {}
            impl BeanInfo for Inner {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Inner>;
            type Link = Get<Inner>;
            type Chain = More<Outer>;
}
            impl BeanInfo for Outer {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Outer>;
            type Link = Get<Outer>;
            type Chain = Stop;
}
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl At<Inner> for App { type Ix = Ix0; }
            fn probe() { let _claim = App::query().inject_resources(Inner::T); }
            ",
                "not an injectable bean",
            );
        }

        fn assert_full_rustc(body: &str, description: &str) {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos());
            let dir = std::env::temp_dir().join(format!("ph-beans-dup-{nanos}"));
            fs::create_dir_all(&dir).expect("probe dir");
            let path = dir.join("probe.rs");
            fs::write(&path, body).expect("probe source");
            let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
            let output = Command::new(rustc)
                .arg("--edition")
                .arg("2024")
                .arg("--crate-type")
                .arg("lib")
                .arg("--error-format")
                .arg("short")
                .arg("--out-dir")
                .arg(&dir)
                .arg("--extern")
                .arg(format!("ph_beans={}", beans_codegen_rlib().display()))
                .arg(&path)
                .output()
                .expect("rustc");
            let stderr = String::from_utf8(output.stderr).expect("stderr utf-8");
            let _ = fs::remove_dir_all(&dir);
            assert!(
                stderr.contains(description),
                "description {description}\n{stderr}"
            );
        }

        pub(super) fn fail(body: &str, description: &str) {
            ErrorTester::source(body)
                .extern_crate("ph_beans", beans_rlib())
                .assert_description(description);
        }

        fn beans_rlib() -> PathBuf {
            newest_beans_lib(false)
        }

        fn beans_codegen_rlib() -> PathBuf {
            newest_beans_lib(true)
        }

        fn newest_beans_lib(rlib_only: bool) -> PathBuf {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let target = std::env::var_os("CARGO_TARGET_DIR")
                .map_or_else(|| manifest.join("../../target"), PathBuf::from);
            let mut found = Vec::new();
            collect_rlib(&target.join("debug/deps"), &mut found);
            let build = target.join("debug/build/ph_beans");
            if let Ok(hashes) = fs::read_dir(build) {
                for hash in hashes.flatten() {
                    collect_rlib(&hash.path().join("out"), &mut found);
                }
            }
            if rlib_only {
                found.retain(|path| path.extension().is_some_and(|ext| ext == "rlib"));
            }
            found
                .into_iter()
                .max_by_key(|path| {
                    path.metadata()
                        .and_then(|meta| meta.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                })
                .expect("ph_beans rlib")
        }

        fn collect_rlib(dir: &std::path::Path, found: &mut Vec<PathBuf>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let meta = entry.metadata().ok();
                let nonempty = meta.as_ref().is_some_and(|item| item.len() > 0);
                if nonempty
                    && name.starts_with("libph_beans-")
                    && (name.ends_with(".rlib") || name.ends_with(".rmeta"))
                {
                    found.push(entry.path());
                }
            }
        }
    }

    mod tree {
        //! Query slice plus module injects and `Tick::update`.
        //!
        //! Rejections live next to this module: a second init, a missing `.build()`,
        //! and `Mut` against an immutable provide.

        use ph_beans::{
            At, BeanInfo, EventInfo, Get, Ix0, Ix1, Marker, Module, Mut, OfInject, OfParent,
            OfProvide, OfProvide1, Proxy, Query, Setup, Target, Tick,
        };

        struct Id(u8);
        struct Keys(u8);
        struct App;
        struct Update;

        impl BeanInfo for Id {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = u8;

            type Proxy = Proxy<Id>;

            type Link = ph_beans::Get<Id>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for Keys {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = u8;

            type Proxy = Proxy<Keys>;

            type Link = ph_beans::Get<Keys>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for App {
            type Provides = (Id, Keys);
            type Injects = ();
            type Beans = (Id, Keys);
            type State = u8;

            type Proxy = Proxy<App>;

            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for Update {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Update>;

            type Link = ph_beans::Get<Update>;
            type Chain = ph_beans::Stop;
        }

        impl EventInfo for Update {
            type Parents = ();
        }

        impl Target for Id {}
        impl Target for Keys {}
        impl Target for App {}
        impl Target for Logic {}
        impl Target for Render {}

        impl At<Id> for App {
            type Ix = Ix0;
        }

        impl At<Keys> for App {
            type Ix = Ix1;
        }

        fn boot(args: (&Id, &mut Keys)) -> u8 {
            let (id, keys) = args;
            id.0.wrapping_add(keys.0)
        }

        impl Setup for App {
            fn setup(self) -> impl Tick<Self> {
                App::query()
                    .inject_resources(Id::T & Keys::MUT)
                    .init_state(boot)
                    .build()
            }
        }

        #[test]
        fn setup_accepts_a_built_pipeline() {
            let _pipeline = App.setup();
        }

        struct Logic(u8);
        struct Render;
        struct UsesApp;
        struct UsesRender;
        struct PairHost;
        struct TakesKeys;

        impl BeanInfo for Logic {
            type Provides = Get<Render>;
            type Injects = Get<App>;
            type Beans = (Get<App>, Get<Render>);
            type State = u8;

            type Proxy = Proxy<Logic>;

            type Link = ph_beans::Get<Logic>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for Render {
            type Provides = ();
            type Injects = Mut<Logic>;
            type Beans = Mut<Logic>;
            type State = u8;

            type Proxy = Proxy<Render>;

            type Link = ph_beans::Get<Render>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for UsesApp {
            type Provides = ();
            type Injects = Get<App>;
            type Beans = Get<App>;
            type State = ();

            type Proxy = Proxy<UsesApp>;

            type Link = ph_beans::Get<UsesApp>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for UsesRender {
            type Provides = ();
            type Injects = Get<Render>;
            type Beans = Get<Render>;
            type State = ();

            type Proxy = Proxy<UsesRender>;

            type Link = ph_beans::Get<UsesRender>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for PairHost {
            type Provides = (Get<Id>, Mut<Keys>);
            type Injects = ();
            type Beans = (Get<Id>, Mut<Keys>);
            type State = ();

            type Proxy = Proxy<PairHost>;

            type Link = ph_beans::Get<PairHost>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for TakesKeys {
            type Provides = ();
            type Injects = Mut<Keys>;
            type Beans = Mut<Keys>;
            type State = ();

            type Proxy = Proxy<TakesKeys>;

            type Link = ph_beans::Get<TakesKeys>;
            type Chain = ph_beans::Stop;
        }

        impl Setup for Render {
            fn setup(self) -> impl Tick<Self> {
                Render::query().init_state_default().build()
            }
        }

        #[test]
        fn default_state_is_ready_before_update() {
            let built = Render::query().init_state_default().build();
            assert_eq!(*built.state(), 0);
        }

        #[test]
        fn a_module_updates_with_only_its_inject() {
            let mut tick = Render.setup();
            let mut logic = Logic(4);
            tick.update(&mut logic);
            assert_eq!(logic.0, 4);
        }

        #[test]
        fn a_child_injects_the_parent_surface() {
            fn parent<C: Module<Logic, OfParent>>() {}
            fn from_inject<C: Module<Logic, OfInject>>() {}
            fn from_provide<C: Module<Logic, OfProvide>>() {}
            fn second<C: Module<PairHost, OfProvide1>>() {}
            parent::<Render>();
            from_inject::<UsesApp>();
            from_provide::<UsesRender>();
            second::<TakesKeys>();
        }

        #[test]
        fn an_event_lists_its_parents() {
            fn parents<E: EventInfo>() {}
            parents::<Update>();
        }
    }

    mod schedule {
        //! `a_single_map_runs` is the manual map.
        //!
        //! `map_0` through `map_15` are `impl_foreach`. Nine and sixteen check the list.

        use core::cell::Cell;

        use ph_beans::{
            At, BeanInfo, Ctx, EventInfo, Init, Ix0, Marker, Pipe, Proxy, Query, Target, Tick,
        };
        use ph_util_foreach::impl_foreach;

        thread_local! {
            static HITS: Cell<u8> = const { Cell::new(0) };
            static STEP: Cell<u8> = const { Cell::new(0) };
        }

        fn idle(_: ()) {}

        struct Sched;
        struct Update;
        struct Graphics;
        struct Main;
        struct Keys;

        impl BeanInfo for Sched {
            type Provides = Update;
            type Injects = ();
            type Beans = Update;
            type State = ();

            type Proxy = Proxy<Sched>;

            type Link = ph_beans::Get<Sched>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for Update {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Update>;

            type Link = ph_beans::Get<Update>;
            type Chain = ph_beans::Stop;
        }

        impl EventInfo for Update {
            type Parents = ();
        }

        impl BeanInfo for Graphics {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Graphics>;

            type Link = ph_beans::Get<Graphics>;
            type Chain = ph_beans::Stop;
        }

        impl EventInfo for Graphics {
            type Parents = Update;
        }

        impl BeanInfo for Main {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Main>;

            type Link = ph_beans::Get<Main>;
            type Chain = ph_beans::Stop;
        }

        impl EventInfo for Main {
            type Parents = Update;
        }

        impl BeanInfo for Keys {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Keys>;

            type Link = ph_beans::Get<Keys>;
            type Chain = ph_beans::Stop;
        }

        impl At<Keys> for Sched {
            type Ix = Ix0;
        }

        impl Target for Sched {}
        impl Target for Keys {}
        impl Target for Update {}
        impl Target for Graphics {}
        impl Target for Main {}

        fn graphics_under_update<A, S, P>(
            _pipeline: &Init<(Graphics, Update), Ctx<Sched, A, S, P>>,
        ) {
        }

        fn bump_graphics() {
            HITS.with(|hits| hits.set(hits.get() + 1));
        }

        fn bump_main() {
            HITS.with(|hits| hits.set(hits.get() + 10));
        }

        fn reset_hits() {
            HITS.with(|hits| hits.set(0));
        }

        fn expect_next(expect: u8) {
            STEP.with(|step| {
                assert_eq!(step.get(), expect);
                step.set(expect + 1);
            });
        }

        fn reset_step() {
            STEP.with(|step| step.set(0));
        }

        fn step_value() -> u8 {
            STEP.with(Cell::get)
        }

        impl_foreach!(n in 0..16 {
            fn paste(map_, n)() {
                expect_next(n);
            }
        });

        #[test]
        fn a_nested_on_replaces_the_event_spine() {
            let pipeline = Sched::query().filter(Update::ON[Graphics::ON]);
            graphics_under_update(&pipeline);
        }

        #[test]
        fn a_wait_lets_the_next_resource_replace_the_claim() {
            let _pipeline = Sched::query()
                .inject_resources(Keys::MUT)
                .filter(Update::ON[Main::ON[Keys::MUT]])
                .inject_resources(Keys::T);
        }

        #[test]
        fn emitted_branch_runs_and_the_other_is_skipped() {
            reset_hits();
            let mut tick = Sched::query()
                .filter(Update::ON[Graphics::ON])
                .map(bump_graphics)
                .filter(Update::ON[Main::ON])
                .map(bump_main)
                .emit(Update::ON[Main::ON])
                .init_state(idle)
                .build();
            Tick::<Sched>::update(&mut tick, ());
            assert_eq!(HITS.with(Cell::get), 10);
        }

        #[test]
        fn a_single_map_runs() {
            reset_step();
            let mut tick = Sched::query().map(map_0).init_state(idle).build();
            Tick::<Sched>::update(&mut tick, ());
            assert_eq!(step_value(), 1);
        }

        #[test]
        fn nine_maps_run_across_two_groups() {
            reset_step();
            let mut tick = Sched::query()
                .map(map_0)
                .map(map_1)
                .map(map_2)
                .map(map_3)
                .map(map_4)
                .map(map_5)
                .map(map_6)
                .map(map_7)
                .map(map_8)
                .init_state(idle)
                .build();
            Tick::<Sched>::update(&mut tick, ());
            assert_eq!(step_value(), 9);
        }

        #[test]
        fn sixteen_maps_fit_in_the_list() {
            reset_step();
            let mut tick = Sched::query()
                .map(map_0)
                .map(map_1)
                .map(map_2)
                .map(map_3)
                .map(map_4)
                .map(map_5)
                .map(map_6)
                .map(map_7)
                .map(map_8)
                .map(map_9)
                .map(map_10)
                .map(map_11)
                .map(map_12)
                .map(map_13)
                .map(map_14)
                .map(map_15)
                .init_state(idle)
                .build();
            Tick::<Sched>::update(&mut tick, ());
            assert_eq!(step_value(), 16);
        }

        #[test]
        fn emit_providers_names_the_provider_event() {
            #[allow(clippy::type_complexity)]
            fn holds_update<A, S, I, L>(
                _pipeline: &Init<
                    (),
                    Ctx<Sched, A, S, Pipe<I, ph_beans::MapList<L, ph_beans::On<Update, ()>, ()>>>,
                >,
            ) {
            }
            let pipeline = Sched::query().emit_providers();
            holds_update(&pipeline);
        }
    }

    mod dyn_view {
        //! Dyn inject: concrete bean claim; cast is attribute / Injects encoding.

        use std::fs;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicU64, Ordering};

        use ph_beans::{At, BeanInfo, DynView, Ix0, Marker, Proxy, Query, Target};
        use ph_test_asm::AsmTester;

        struct Draw(u8);
        struct App;

        impl BeanInfo for Draw {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = ();

            type Proxy = Proxy<Draw>;

            type Link = ph_beans::Get<Draw>;
            type Chain = ph_beans::Stop;
        }

        impl BeanInfo for App {
            type Provides = (Draw,);
            type Injects = ();
            type Beans = (Draw,);
            type State = u8;

            type Proxy = Proxy<App>;

            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
        }

        impl At<Draw> for App {
            type Ix = Ix0;
        }

        impl Target for Draw {}
        impl Target for App {}

        trait Painter {
            fn paint(&self) -> u8;
        }

        impl Painter for Draw {
            fn paint(&self) -> u8 {
                self.0
            }
        }

        impl<T: Painter> DynView<T> for dyn Painter {}

        // Prototype without attr encoding: claim is still `Draw::T` / `Draw::MUT`
        // (`ArgsView` = `&Draw`). Cast inject makes only the final arg `&dyn`
        // (design: like `fn process(p: &dyn Platform)` with a concrete `Unix`).
        fn boot(draw: &Draw) -> u8 {
            let view: &dyn Painter = draw;
            view.paint()
        }

        fn boot_mut(draw: &mut Draw) -> u8 {
            let view: &mut dyn Painter = draw;
            view.paint()
        }

        #[test]
        fn a_shared_inject_uses_the_ordinary_bean_claim() {
            let _built = App::query()
                .inject_resources(Draw::T)
                .init_state(boot)
                .build();
        }

        #[test]
        fn an_exclusive_inject_uses_the_ordinary_bean_claim() {
            let _built = App::query()
                .inject_resources(Draw::MUT)
                .init_state(boot_mut)
                .build();
        }

        #[test]
        fn a_monomorphized_dyn_view_has_no_virtual_call() {
            let path = write_probe(
                "dyn_inject_probe.rs",
                r"
            trait Painter {
                fn paint(&self) -> u32;
            }
            struct Draw(u32);
            impl Painter for Draw {
                fn paint(&self) -> u32 {
                    self.0
                }
            }

            #[unsafe(no_mangle)]
            pub fn on_tick(draws: &[Draw]) -> u32 {
                let mut total = 0u32;
                let mut index = 0usize;
                while index < draws.len() {
                    let view: &dyn Painter = &draws[index];
                    total = total.wrapping_add(view.paint());
                    index += 1;
                }
                total
            }
            ",
            );
            AsmTester::new(path).function("on_tick").hot_loop(|hot| {
                hot.assert_no_virtual_calls();
            });
        }

        fn write_probe(name: &str, source: &str) -> PathBuf {
            static TICK: AtomicU64 = AtomicU64::new(0);
            let n = TICK.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("ph-beans-dyn-{n}-{}", std::process::id()));
            fs::create_dir_all(&dir).expect("temp dir");
            let path = dir.join(name);
            fs::write(&path, source).expect("probe");
            path
        }
    }

    mod errors {
        //! Compile-fail pairs for the derive.
        //!
        //! A missing provide is `a_missing_provide_is_rejected`.
        //! The program that compiles is the tree test.

        use ph_proc_macro_dylib::proc_macro_library;
        use ph_test_errors::ErrorTester;

        #[test]
        fn duplicate_provider_is_rejected() {
            fail(
                "
            #[derive(Component)]
            #[provider(Count, Count)]
            struct Parent;
            ",
                "duplicate bean in `provider`",
            );
        }

        #[test]
        fn duplicate_inject_is_rejected() {
            fail(
                "
            #[derive(Component)]
            #[inject(Count, Count)]
            struct Child;
            ",
                "duplicate bean in `inject`",
            );
        }

        #[test]
        fn a_bean_in_both_lists_is_rejected() {
            fail(
                "
            #[derive(Component)]
            #[provider(Count)]
            #[inject(Count)]
            struct Child;
            ",
                "bean is in both `provider` and `inject`",
            );
        }

        #[test]
        fn five_providers_are_rejected() {
            fail(
                "
            #[derive(Component)]
            #[provider(A, B, C, D, E)]
            struct Parent;
            ",
                "accepts at most 4 beans",
            );
        }

        #[test]
        fn a_generic_component_is_rejected() {
            fail(
                "
            #[derive(Component)]
            struct Holder<T>(T);
            ",
                "does not support generic",
            );
        }

        #[test]
        fn four_injects_are_rejected() {
            fail(
                "
            #[derive(Component)]
            #[inject(A, B, C, D)]
            struct Child;
            ",
                "accepts at most 3 beans",
            );
        }

        fn fail(body: &str, description: &str) {
            let source = format!("use ph_beans_derive::Component;\n{body}\n");
            ErrorTester::source(source)
                .extern_crate("ph_beans_derive", proc_macro_library("ph_beans_derive"))
                .assert_description(description);
        }
    }

    mod reject {
        //! Each probe is one rejected call. The plain bean shape is `Id` in `proxy`.

        #[test]
        fn exclusive_against_a_shared_provide_is_rejected() {
            super::proxy::fail(
            "
            use ph_beans::{BeanInfo, Proxy, FromParent, Get, Mut, OfProvide, Proxied, Target};
            struct Logic;
            struct App;
            impl BeanInfo for Logic {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Logic>;
            type Link = ph_beans::Get<Logic>;
            type Chain = ph_beans::Stop;
}
            impl Target for Logic {}
            impl BeanInfo for App {
                type Provides = Get<Logic>; type Injects = (); type Beans = Get<Logic>; type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            fn probe() { fn mount<C: FromParent<App, OfProvide>>() {} mount::<Mut<Logic>>(); }
            ",
            "Mut<Logic>",
        );
        }

        #[test]
        fn a_missing_provide_is_rejected() {
            super::proxy::fail(
                "
            use ph_beans::{BeanInfo, Proxy, Get, Marker, Proxied, Query, Target};
            struct Id;
            impl BeanInfo for Id {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Id>;
            type Link = ph_beans::Get<Id>;
            type Chain = ph_beans::Stop;
}
            impl Target for Id {}
            struct App;
            impl BeanInfo for App {
                type Provides = (Id,); type Injects = (); type Beans = (Id,); type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            fn probe() { let _claim = App::query().inject_resources(Id::T); }
            ",
                "not on the proxy list",
            );
        }

        #[test]
        fn a_second_init_state_is_rejected() {
            super::proxy::fail(
                "
            use ph_beans::{BeanInfo, Proxy, Get, Proxied, Query, Target};
            struct App;
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = u8;
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl Target for App {}
            fn boot(_: ()) -> u8 { 1 }
            fn probe() { let _twice = App::query().init_state(boot).init_state(boot); }
            ",
                "method `init_state` exists",
            );
        }

        #[test]
        fn a_second_default_init_is_rejected() {
            super::proxy::fail(
                "
            use ph_beans::{BeanInfo, Proxy, Get, Proxied, Query, Target};
            struct App;
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = u8;
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl Target for App {}
            fn probe() { let _twice = App::query().init_state_default().init_state_default(); }
            ",
                "method `init_state_default` exists",
            );
        }

        #[test]
        fn a_live_mut_blocks_the_next_inject() {
            super::proxy::fail(
            "
            use ph_beans::{At, BeanInfo, Proxy, EventInfo, Get, Ix0, Marker, Proxied, Query, Target};
            struct Keys;
            struct Main;
            struct App;
            impl BeanInfo for Keys {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Keys>;
            type Link = ph_beans::Get<Keys>;
            type Chain = ph_beans::Stop;
}
            impl Target for Keys {}
            impl BeanInfo for Main {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<Main>;
            type Link = ph_beans::Get<Main>;
            type Chain = ph_beans::Stop;
}
            impl EventInfo for Main { type Parents = (); }
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = ();
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl At<Keys> for App { type Ix = Ix0; }
            fn probe() {
                let _blocked = App::query()
                    .inject_resources(Keys::MUT)
                    .filter(Main::ON[Keys::T])
                    .inject_resources(Keys::T);
            }
            ",
            "method `inject_resources` exists",
        );
        }

        #[test]
        fn setup_without_build_is_rejected() {
            super::proxy::fail(
                "
            use ph_beans::{BeanInfo, Proxy, Get, Proxied, Query, Setup, Target, Tick};
            struct App;
            impl BeanInfo for App {
                type Provides = (); type Injects = (); type Beans = (); type State = u8;
            
            type Proxy = Proxy<App>;
            type Link = ph_beans::Get<App>;
            type Chain = ph_beans::Stop;
}
            impl Target for App {}
            fn boot(_: ()) -> u8 { 1 }
            impl Setup for App {
                fn setup(self) -> impl Tick<Self> { App::query().init_state(boot) }
            }
            ",
                "Tick",
            );
        }
    }
}
