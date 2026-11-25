//! [`Hand`] for flat tuples of one to eight slots: one impl per length
//! (rules 11–12). Each hands its children on, in order.

use crate::context::list::{Hand, Visit};

// Generics: `R` resources, `E0` child slot.
impl<R, E0> Hand<R> for (E0,)
where
    E0: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E1` child slots.
impl<R, E0, E1> Hand<R> for (E0, E1)
where
    E0: Hand<R>,
    E1: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E2` child slots.
impl<R, E0, E1, E2> Hand<R> for (E0, E1, E2)
where
    E0: Hand<R>,
    E1: Hand<R>,
    E2: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
        self.2.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E3` child slots.
impl<R, E0, E1, E2, E3> Hand<R> for (E0, E1, E2, E3)
where
    E0: Hand<R>,
    E1: Hand<R>,
    E2: Hand<R>,
    E3: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
        self.2.hand(op, res);
        self.3.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E4` child slots.
impl<R, E0, E1, E2, E3, E4> Hand<R> for (E0, E1, E2, E3, E4)
where
    E0: Hand<R>,
    E1: Hand<R>,
    E2: Hand<R>,
    E3: Hand<R>,
    E4: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
        self.2.hand(op, res);
        self.3.hand(op, res);
        self.4.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E5` child slots.
impl<R, E0, E1, E2, E3, E4, E5> Hand<R> for (E0, E1, E2, E3, E4, E5)
where
    E0: Hand<R>,
    E1: Hand<R>,
    E2: Hand<R>,
    E3: Hand<R>,
    E4: Hand<R>,
    E5: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
        self.2.hand(op, res);
        self.3.hand(op, res);
        self.4.hand(op, res);
        self.5.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E6` child slots.
impl<R, E0, E1, E2, E3, E4, E5, E6> Hand<R> for (E0, E1, E2, E3, E4, E5, E6)
where
    E0: Hand<R>,
    E1: Hand<R>,
    E2: Hand<R>,
    E3: Hand<R>,
    E4: Hand<R>,
    E5: Hand<R>,
    E6: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
        self.2.hand(op, res);
        self.3.hand(op, res);
        self.4.hand(op, res);
        self.5.hand(op, res);
        self.6.hand(op, res);
    }
}

// Generics: `R` resources, `E0`…`E7` child slots.
impl<R, E0, E1, E2, E3, E4, E5, E6, E7> Hand<R> for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    E0: Hand<R>,
    E1: Hand<R>,
    E2: Hand<R>,
    E3: Hand<R>,
    E4: Hand<R>,
    E5: Hand<R>,
    E6: Hand<R>,
    E7: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.0.hand(op, res);
        self.1.hand(op, res);
        self.2.hand(op, res);
        self.3.hand(op, res);
        self.4.hand(op, res);
        self.5.hand(op, res);
        self.6.hand(op, res);
        self.7.hand(op, res);
    }
}
