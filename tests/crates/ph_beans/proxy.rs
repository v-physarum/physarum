//! Proxy wrapper, [`DefaultProxy`], init forwarding, and proxy events.
//!
//! [`DefaultProxy`]: ph_beans::DefaultProxy

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use core::cell::Cell;

use ph_test_errors::ErrorTester;

use ph_beans::{
    At, BeanInfo, ClaimVia, Ctx, DefaultProxy, EventInfo, FromChain, FromProxy, Fwd, Get, Init,
    InitPath, Ix0, Ix1, Live, Marker, More, Mut, Own, Pipe, Proxied, ProxyInfo, Query, Stop,
    Target, Tick, Via, WithInit,
};

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
struct Score;
struct Shell;
struct Update;
struct GameTick;
struct Gate;
struct Loud;
struct Main;
struct Host;
struct Plain;

impl BeanInfo for Id {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Id {}
impl Proxied for Id {
    type Proxy = DefaultProxy<Id>;
    type Link = Get<DefaultProxy<Id>>;
}

impl BeanInfo for Score {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Score {}
impl Proxied for Score {
    type Proxy = DefaultProxy<Score>;
    type Link = Get<DefaultProxy<Score>>;
}

impl BeanInfo for Shell {
    type Provides = (Id, GameTick);
    type Injects = ();
    type Beans = (Id, GameTick);
    type State = ();
}
impl Target for Shell {}
impl Proxied for Shell {
    type Proxy = DefaultProxy<Shell>;
    type Link = Get<DefaultProxy<Shell>>;
}
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
}
impl ProxyInfo for Update {
    type Targets = GameTick;
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
}
impl Target for GameTick {}
impl Proxied for GameTick {
    type Proxy = Update;
    type Link = Get<Update>;
}
impl EventInfo for GameTick {
    type Parents = ();
}

impl BeanInfo for Gate {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl ProxyInfo for Gate {
    type Targets = Loud;
    type Link = Mut<Gate>;
    type Chain = Stop;
}

impl BeanInfo for Loud {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Loud {}
impl Proxied for Loud {
    type Proxy = Gate;
    type Link = Mut<Gate>;
}

impl BeanInfo for Main {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Main {}
impl Proxied for Main {
    type Proxy = DefaultProxy<Main>;
    type Link = Get<DefaultProxy<Main>>;
}
impl EventInfo for Main {
    type Parents = ();
}

impl BeanInfo for Host {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Host {}
impl Proxied for Host {
    type Proxy = DefaultProxy<Host>;
    type Link = Get<DefaultProxy<Host>>;
}

impl BeanInfo for Plain {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Plain {}
impl Proxied for Plain {
    type Proxy = DefaultProxy<Plain>;
    type Link = Get<DefaultProxy<Plain>>;
}
impl EventInfo for Plain {
    type Parents = ();
}

struct Core;
struct Inner;
struct Mid;
struct Outer;
struct Prize;
struct Bonus;
struct Alarm;
struct Room;

impl BeanInfo for Core {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u8;
}
impl Target for Core {}
impl Proxied for Core {
    type Proxy = Inner;
    type Link = Get<Inner>;
}
impl EventInfo for Core {
    type Parents = ();
}

impl BeanInfo for Inner {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl ProxyInfo for Inner {
    type Targets = Core;
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
}
impl ProxyInfo for Mid {
    type Targets = Inner;
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
}
impl ProxyInfo for Outer {
    type Targets = Mid;
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

impl BeanInfo for Prize {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Prize {}
impl Proxied for Prize {
    type Proxy = DefaultProxy<Prize>;
    type Link = Get<DefaultProxy<Prize>>;
}

impl BeanInfo for Bonus {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Bonus {}
impl Proxied for Bonus {
    type Proxy = DefaultProxy<Bonus>;
    type Link = Get<DefaultProxy<Bonus>>;
}

impl BeanInfo for Alarm {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Alarm {}
impl Proxied for Alarm {
    type Proxy = DefaultProxy<Alarm>;
    type Link = Get<DefaultProxy<Alarm>>;
}
impl EventInfo for Alarm {
    type Parents = ();
}

impl BeanInfo for Room {
    type Provides = (Core,);
    type Injects = ();
    type Beans = (Core,);
    type State = ();
}
impl Target for Room {}
impl Proxied for Room {
    type Proxy = DefaultProxy<Room>;
    type Link = Get<DefaultProxy<Room>>;
}
impl At<Core> for Room {
    type Ix = Ix0;
}

trait Same<T> {}
impl<T> Same<T> for T {}

trait Through<P> {}
impl<H, P, L> Through<P> for Pipe<WithInit<Fwd<P, H>>, L> {}

type DefaultWrap<S, T> = Init<(), Ctx<Shell, Live<Via<DefaultProxy<Id>, Get<Id>>>, S, T>>;
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
    shared::<<GameTick as Proxied>::Link>();
    exclusive::<<Loud as Proxied>::Link>();
    shared::<<Update as ProxyInfo>::Link>();
    exclusive::<<Gate as ProxyInfo>::Link>();
}

#[test]
fn the_target_sees_the_proxy_provides() {
    fn via_proxy<R: ClaimVia<GameTick, FromProxy>>() {}
    via_proxy::<Get<Score>>();
    let _surface = GameTick::query().inject_resources(Score::T);
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
    assert_init::<_, _, _, _, DefaultProxy<Host>>(&pipeline);
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
    fn at_mid<R: ClaimVia<Core, (FromChain, Own)>>() {}
    fn at_outer<R: ClaimVia<Core, (FromChain, (FromProxy, Own))>>() {}
    at_mid::<Get<Prize>>();
    at_outer::<Get<Bonus>>();
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

#[test]
fn emitting_any_proxy_in_the_chain_runs_the_bean() {
    reset();
    let mut tick = Room::query()
        .filter(Core::ON)
        .map(hit)
        .emit(Inner::ON)
        .init_state(idle)
        .build();
    Tick::<Room>::update(&mut tick, ());
    assert_eq!(HITS.with(Cell::get), 1);

    reset();
    let mut tick = Room::query()
        .filter(Core::ON)
        .map(hit)
        .emit(Mid::ON)
        .init_state(idle)
        .build();
    Tick::<Room>::update(&mut tick, ());
    assert_eq!(HITS.with(Cell::get), 1);

    reset();
    let mut tick = Room::query()
        .filter(Core::ON)
        .map(hit)
        .emit(Outer::ON)
        .init_state(idle)
        .build();
    Tick::<Room>::update(&mut tick, ());
    assert_eq!(HITS.with(Cell::get), 1);
}

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
        use ph_beans::{At, BeanInfo, Ix0, Marker, ProxyInfo, Query, Stop, Target};
        struct Tick;
        struct Update;
        struct App;
        impl BeanInfo for Tick {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl Target for Tick {}
        impl BeanInfo for Update {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl ProxyInfo for Update {
            type Targets = Tick; type Link = ph_beans::Get<Update>; type Chain = Stop;
        }
        impl BeanInfo for App {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl At<Update> for App { type Ix = Ix0; }
        fn probe() { let _claim = App::query().inject_resources(Update::MUT); }
        ",
        "ClaimVia",
    );
}

#[test]
fn injecting_a_proxy_provide_is_rejected() {
    fail(
        "
        use ph_beans::{BeanInfo, FromParent, Get, OfProvide, ProxyInfo, Stop, Target};
        struct Tick;
        struct Update;
        struct App;
        impl BeanInfo for Tick {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl Target for Tick {}
        impl BeanInfo for Update {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl ProxyInfo for Update {
            type Targets = Tick; type Link = Get<Update>; type Chain = Stop;
        }
        impl BeanInfo for App {
            type Provides = Get<Update>; type Injects = (); type Beans = Get<Update>; type State = ();
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
        use ph_beans::{At, BeanInfo, Ix0, ProxyInfo, Stop, Target};
        struct Tick;
        struct Update;
        struct App;
        impl BeanInfo for Tick {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl Target for Tick {}
        impl BeanInfo for Update {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl ProxyInfo for Update {
            type Targets = Tick; type Link = ph_beans::Get<Update>; type Chain = Stop;
        }
        impl BeanInfo for App {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
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
        use ph_beans::{At, BeanInfo, Get, Ix0, Marker, More, Proxied, ProxyInfo, Query, Stop};
        struct Tick;
        struct Inner;
        struct Outer;
        struct App;
        impl BeanInfo for Tick {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl ph_beans::Target for Tick {}
        impl BeanInfo for Inner {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl ProxyInfo for Inner {
            type Targets = Tick; type Link = Get<Inner>; type Chain = More<Outer>;
        }
        impl BeanInfo for Outer {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl ProxyInfo for Outer {
            type Targets = Inner; type Link = Get<Outer>; type Chain = Stop;
        }
        impl Proxied for Tick { type Proxy = Inner; type Link = Get<Inner>; }
        impl BeanInfo for App {
            type Provides = (); type Injects = (); type Beans = (); type State = ();
        }
        impl At<Inner> for App { type Ix = Ix0; }
        fn probe() { let _claim = App::query().inject_resources(Inner::T); }
        ",
        "ClaimVia",
    );
}

fn fail(body: &str, description: &str) {
    ErrorTester::source(body)
        .extern_crate("ph_beans", beans_rlib())
        .assert_description(description);
}

fn beans_rlib() -> PathBuf {
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
        if name.starts_with("libph_beans-") && name.ends_with(".rlib") {
            found.push(entry.path());
        }
    }
}
