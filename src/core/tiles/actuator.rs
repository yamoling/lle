//! Logic shared by the tiles that pulse lifts: `Button`, `Lever` and `PressurePlate`.

use crate::agent::Colour;

/// Whether an actuator restricted to `authorized` (if any) admits an agent of `colour`.
pub(crate) fn authorizes(authorized: Option<Colour>, colour: Colour) -> bool {
    authorized.is_none_or(|auth| auth == colour)
}
