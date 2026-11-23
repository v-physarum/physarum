//! Proxy wrapper, default [`Pass`], init forwarding, and proxy events.
//!
//! [`Pass`]: ph_beans::Pass

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use core::cell::Cell;

use ph_beans::{
    At, BeanInfo, ClaimVia, Ctx, EventInfo, FromProxy, Fwd, Get, Init, InitPath, Ix0, Ix1, Live,
    Marker, Mut, Pass, Pipe, Proxied, ProxyInfo, Query, Target, Tick, Via, WithInit,
};

thread_local! {
    static HITS: Cell<u8> = const { Cell::new(0) };
}

fn idle(_: ()) {}

fn hit() {
    HITS.with(|hits| hits.set(hits.get() + 1));
}

fn reset() {
    HITS.with(|hits| hits.set(0));
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
    type Proxy = Pass<Id>;
    type Link = Get<Pass<Id>>;
}

impl BeanInfo for Score {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Score {}
impl Proxied for Score {
    type Proxy = Pass<Score>;
    type Link = Get<Pass<Score>>;
}

impl BeanInfo for Shell {
    type Provides = (Id, GameTick);
    type Injects = ();
    type Beans = (Id, GameTick);
    type State = ();
}
impl Target for Shell {}
impl Proxied for Shell {
    type Proxy = Pass<Shell>;
    type Link = Get<Pass<Shell>>;
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
}
impl At<Score> for Update {
    type Ix = Ix0;
}
impl EventInfo for Update {
    type Parents = Main;
}
impl InitPath<GameTick> for Update {
    fn install<H>(handler: H) -> Fwd<Self, H> {
        Fwd::new(handler)
    }

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
    type Proxy = Pass<Main>;
    type Link = Get<Pass<Main>>;
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
    type Proxy = Pass<Host>;
    type Link = Get<Pass<Host>>;
}

impl BeanInfo for Plain {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}
impl Target for Plain {}
impl Proxied for Plain {
    type Proxy = Pass<Plain>;
    type Link = Get<Pass<Plain>>;
}
impl EventInfo for Plain {
    type Parents = ();
}

trait Same<T> {}
impl<T> Same<T> for T {}

trait Through<P> {}
impl<H, P, L> Through<P> for Pipe<WithInit<Fwd<P, H>>, L> {}

type PassWrap<S, T> = Init<(), Ctx<Shell, Live<Via<Pass<Id>, Get<Id>>>, S, T>>;
type UpdateWrap<S, T> = Init<(), Ctx<Shell, Live<Via<Update, Get<GameTick>>>, S, T>>;

fn assert_default<S, T>(_: &PassWrap<S, T>) {}

fn assert_explicit<S, T>(_: &UpdateWrap<S, T>) {}

fn assert_init<E, A, S, T, P>(_: &Init<E, Ctx<Host, A, S, T>>)
where
    T: Through<P>,
{
}

#[test]
fn an_unset_proxy_wraps_with_pass() {
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
fn init_of_an_unset_proxy_goes_through_pass() {
    let pipeline = Host::query().init_state(idle);
    assert_init::<_, _, _, _, Pass<Host>>(&pipeline);
}

#[test]
fn init_default_comes_from_the_explicit_proxy() {
    let built = GameTick::query().init_state_default().build();
    assert_eq!(*built.state(), 7);
}
