use ph_util_foreach::impl_foreach;

impl_foreach!(range = 0..CASE_LEN, body = {
    pub struct Ix + #;
});
