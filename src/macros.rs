///
/// A macro for creating a new id type.
/// The resulting type is a wrapper for uuid:Uuid.
///
/// An ArenaId and a PlayerId are ultimately two different types,
/// even if they're currently both encoded as uuids.
///
/// As such, this macro should be used in place of type aliasing uuid::Uuid.
///
#[macro_export]
macro_rules! new_id_type {
    ($ident: ident) => {
        #[derive(
            Debug,
            Hash,
            Copy,
            Clone,
            PartialEq,
            Eq,
            serde_derive::Serialize,
            serde_derive::Deserialize,
        )]
        pub struct $ident(uuid::Uuid);

        impl Default for $ident {
            fn default() -> Self {
                Self(uuid::Uuid::new_v4())
            }
        }

        impl std::fmt::Display for $ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::ops::Deref for $ident {
            type Target = uuid::Uuid;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}
