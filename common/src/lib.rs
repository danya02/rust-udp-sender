mod magic;
mod messages;

/// Make a random name for an object.
/// 
/// Uses the `petname` crate for human-readable names.
pub fn make_name() -> String {
    petname::petname(3, "-")
}