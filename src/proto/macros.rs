use super::Protocol;
use Guid;

/// Implements the `Protocol` trait for a type.
/// Also marks the type as not sync and not send.
pub macro impl_proto (
    protocol $p:ident {
        GUID = $a:expr, $b:expr, $c:expr, $d:expr;
    }
) {
    impl Protocol for $p {
        const GUID: Guid = Guid::from_values($a, $b, $c, $d);
    }

    // Most UEFI functions expect to be called on the bootstrap processor.
    impl !Send for $p {}
    // Most UEFI functions do not support multithreaded access.
    impl !Sync for $p {}
}
