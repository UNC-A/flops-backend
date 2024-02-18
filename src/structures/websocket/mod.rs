pub mod actions;
pub mod events;



pub fn if_false(i: &bool) -> bool{
    !*i
}
pub fn is_none<T>(i: &Option<T>) -> bool{
    i.is_none()
}

