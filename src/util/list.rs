pub trait AddItem {
    type Add<T>;

    fn add_item<T>(self, item: T) -> Self::Add<T>;
}

pub struct List<A>(A);

impl AddItem for List<()> {
    type Add<T> = (T,);

    fn add_item<T>(self, item: T) -> Self::Add<T> {
        (item,)
    }
}

impl<A0, A1> AddItem for (A0, A1) {
    type Add<T> = (A0, T);

    fn add_item<T>(self, item: T) -> Self::Add<T> {
        (self.0, item)
    }
}

impl<A0, A1, A2> AddItem for (A0, A1, A2) {
    type Add<T> = (A0, A1, T);

    fn add_item<T>(self, item: T) -> Self::Add<T> {
        (self.0, self.1, item)
    }
}
