pub mod magic;
pub mod messages;
pub mod networking;

type DecodeError = rmp_serde::decode::Error;

/// Make a random name for an object.
/// 
/// Uses the `petname` crate for human-readable names.
pub fn make_name() -> String {
    petname::petname(3, "-")
}