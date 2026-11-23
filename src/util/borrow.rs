pub trait Take<I> {
    type Out;
    type Rest;

    fn take(self) -> (Self::Out, Self::Rest);
}

impl<A0, A1> Take<Ix0> for (A0, A1) {
    type Out = A0;
    type Rest = (Off, A1);

    fn take(self) -> (Self::Out, Self::Rest) {
        (self.0, (Off, self.1))
    }
}

impl<A0, A1> Take<Ix1> for (A0, A1) {
    type Out = A1;
    type Rest = (A0, Off);

    fn take(self) -> (Self::Out, Self::Rest) {
        (self.1, (self.0, Off))
    }
}

pub trait TakeMany<M, Out> {
    fn take_many(self) -> Out;
}

impl<M, T> TakeMany<M, ()> for T {
    fn take_many(self) {}
}

impl<M, T, O0> TakeMany<M, (O0,)> for T
where
    T: Take<M, Out = O0>,
{
    fn take_many(self) -> (O0,) {
        let (o0, _) = self.take();
        (o0,)
    }
}

impl<M, T, O0, O1> TakeMany<M, (O0, O1)> for T
where
    T: Take<M, Out = O0>,
    T::Rest: TakeMany<M, (O1,)>, // <-- Шлейфа больше нет!
{
    fn take_many(self) -> (O0, O1) {
        let (o0, rest) = self.take();
        let (o1,) = rest.take_many();
        (o0, o1)
    }
}

impl<M, T, O0, O1, O2> TakeMany<M, (O0, O1, O2)> for T
where
    T: Take<M, Out = O0>,
    T::Rest: TakeMany<M, (O1, O2)>,
{
    fn take_many(self) -> (O0, O1, O2) {
        let (o0, rest) = self.take();
        let (o1, o2) = rest.take_many();
        (o0, o1, o2)
    }
}

#[diagnostic::on_unimplemented(message = "`{Self}` does not provide `{P}`")]
pub trait At<P> {
    type Ix;
}

pub struct Ix0;

pub struct Ix1;

pub struct Ix2;

pub struct Ix3;

pub struct Off;
