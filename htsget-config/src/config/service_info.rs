//! Service info configuration.
//!

use crate::error::{Error, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, to_value, Value};
use std::collections::HashMap;

/// Create the package info used to populate the service info. This uses the `CARGO_PKG_*` environment
/// variables for information. A macro allows dependent code to use its own package information.
#[macro_export]
macro_rules! package_info {
  () => {
    $crate::config::service_info::PackageInfo::new(
      format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
      format!(env!("CARGO_PKG_NAME")),
      format!(env!("CARGO_PKG_VERSION")),
      format!(env!("CARGO_PKG_DESCRIPTION")),
      format!(env!("CARGO_PKG_REPOSITORY")),
    )
  };
}

/// Package info used to create the service info.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct PackageInfo {
  id: String,
  name: String,
  version: String,
  description: String,
  documentation_url: String,
}

impl PackageInfo {
  /// Create a new package info.
  pub fn new(
    id: String,
    name: String,
    version: String,
    description: String,
    documentation_url: String,
  ) -> Self {
    Self {
      id,
      name,
      version,
      description,
      documentation_url,
    }
  }
}

/// Fields that can be captured in the service info. These are optional
/// to be able to distinguish between user-specified values and defaults.
/// Required fields like `id` get filled in later when converting to
/// `ServiceInfo`. Any custom fields are captured in `fields`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceInfoFields {
  organization: Organization,
  #[serde(skip_serializing_if = "Option::is_none")]
  created_at: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  updated_at: Option<String>,
  #[serde(flatten)]
  fields: HashMap<String, Value>,
}

/// Organization info for the service info.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct Organization {
  name: Option<String>,
  url: Option<String>,
}

impl TryFrom<ServiceInfoFields> for ServiceInfo {
  type Error = Error;

  fn try_from(mut fields: ServiceInfoFields) -> Result<Self> {
    // Set the required fields, except version, name and id, which gets set later by dependent code.
    fields.organization.name.get_or_insert_default();
    fields.organization.url.get_or_insert_default();

    // These are optional but nice to default to current time.
    fields
      .created_at
      .get_or_insert_with(|| Utc::now().to_rfc3339());
    fields
      .updated_at
      .get_or_insert_with(|| Utc::now().to_rfc3339());

    let fields: HashMap<String, Value> = from_value(to_value(fields)?)?;

    let err_msg = |invalid_key| format!("reserved service info field `{}`", invalid_key);
    if fields.contains_key("type") {
      return Err(Error::ParseError(err_msg("type")));
    }
    if fields.contains_key("htsget") {
      return Err(Error::ParseError(err_msg("htsget")));
    }

    Ok(Self::new(fields))
  }
}

/// Service info config.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, try_from = "ServiceInfoFields")]
pub struct ServiceInfo(HashMap<String, Value>);

impl ServiceInfo {
  /// Create a service info.
  pub fn new(fields: HashMap<String, Value>) -> Self {
    Self(fields)
  }

  /// Insert the value if it does not already exist.
  pub fn entry_or_insert(&mut self, key: String, value: Value) -> &mut Value {
    self.0.entry(key).or_insert(value)
  }

  /// Set the fields from the package info if they have not already been set.
  pub fn set_from_package_info(&mut self, info: PackageInfo) -> Result<()> {
    let mut package_info: HashMap<String, Value> = from_value(to_value(info)?)?;

    package_info.extend(self.0.drain());
    self.0 = package_info;

    Ok(())
  }

  /// Get the inner value.
  pub fn into_inner(self) -> HashMap<String, Value> {
    self.0
  }
}

impl AsRef<HashMap<String, Value>> for ServiceInfo {
  fn as_ref(&self) -> &HashMap<String, Value> {
    &self.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::tests::test_serialize_and_deserialize;
  use crate::config::Config;
  use serde_json::json;

  #[test]
  fn service_info() {
    test_serialize_and_deserialize(
      r#"
      service_info.environment = "dev"
      service_info.organization = { name = "name", url = "https://example.com/" }
      "#,
      HashMap::from_iter(vec![
        ("environment".to_string(), json!("dev")),
        (
          "organization".to_string(),
          json!({ "name": "name", "url": "https://example.com/" }),
        ),
      ]),
      |result: Config| result.service_info.0,
    );
  }
}
