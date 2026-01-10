//! Typed ID wrappers for compile-time type safety.
//!
//! These types wrap UUIDs to prevent accidental mixing of different entity IDs.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::{Database, Decode, Encode, Type};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Macro to generate typed ID wrappers with common trait implementations.
macro_rules! typed_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new random ID.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Creates an ID from an existing UUID.
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the inner UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(Uuid::parse_str(s).expect("Invalid UUID string"))
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(Uuid::parse_str(&s).expect("Invalid UUID string"))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0.to_string()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                Uuid::parse_str(&s)
                    .map(Self)
                    .map_err(serde::de::Error::custom)
            }
        }

        // SQLx integration for reading from database
        impl<'r, DB: Database> Decode<'r, DB> for $name
        where
            String: Decode<'r, DB>,
        {
            fn decode(
                value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let s = String::decode(value)?;
                Uuid::parse_str(&s)
                    .map(Self)
                    .map_err(|e| e.into())
            }
        }

        // SQLx integration for writing to database
        impl<'q, DB: Database> Encode<'q, DB> for $name
        where
            String: Encode<'q, DB>,
        {
            fn encode_by_ref(
                &self,
                buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
            ) -> sqlx::encode::IsNull {
                self.0.to_string().encode_by_ref(buf)
            }
        }

        impl<DB: Database> Type<DB> for $name
        where
            String: Type<DB>,
        {
            fn type_info() -> <DB as Database>::TypeInfo {
                String::type_info()
            }

            fn compatible(ty: &<DB as Database>::TypeInfo) -> bool {
                String::compatible(ty)
            }
        }
    };
}

// Define all typed IDs
typed_id!(UserId, "Unique identifier for a user.");
typed_id!(AttendanceId, "Unique identifier for an attendance record.");
typed_id!(BreakRecordId, "Unique identifier for a break record.");
typed_id!(LeaveRequestId, "Unique identifier for a leave request.");
typed_id!(OvertimeRequestId, "Unique identifier for an overtime request.");
typed_id!(HolidayId, "Unique identifier for a holiday.");
typed_id!(WeeklyHolidayId, "Unique identifier for a weekly holiday.");
typed_id!(HolidayExceptionId, "Unique identifier for a holiday exception.");
typed_id!(AuditLogId, "Unique identifier for an audit log entry.");
