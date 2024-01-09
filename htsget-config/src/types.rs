use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind::Other;
use std::{fmt, io, result};

use http::HeaderMap;
use noodles::core::region::Interval as NoodlesInterval;
use noodles::core::Position;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::instrument;

use crate::error::Error;
use crate::error::Error::ParseError;
use crate::resolver::object::ObjectType;

pub type Result<T> = result::Result<T, HtsGetError>;

/// An enumeration with all the possible formats.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub enum Format {
  #[serde(alias = "bam", alias = "BAM")]
  Bam,
  #[serde(alias = "cram", alias = "CRAM")]
  Cram,
  #[serde(alias = "vcf", alias = "VCF")]
  Vcf,
  #[serde(alias = "bcf", alias = "BCF")]
  Bcf,
}

/// The type of key of the file.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
  File,
  Index,
}

impl KeyType {
  /// Get the key type from an ending.
  pub fn from_ending<K: AsRef<str>>(key: K) -> KeyType {
    if key.as_ref().ends_with(Format::Bam.index_file_ending())
      || key.as_ref().ends_with(Format::Vcf.index_file_ending())
      || key.as_ref().ends_with(Format::Cram.index_file_ending())
      || key.as_ref().ends_with(Format::Vcf.index_file_ending())
      || key.as_ref().ends_with(".bam.gzi")
      || key.as_ref().ends_with(".vcf.gz.gzi")
      || key.as_ref().ends_with(".bcf.gzi")
    {
      Self::Index
    } else {
      Self::File
    }
  }
}

/// Todo allow these to be configurable.
impl Format {
  pub fn file_ending(&self) -> &str {
    match self {
      Format::Bam => ".bam",
      Format::Cram => ".cram",
      Format::Vcf => ".vcf.gz",
      Format::Bcf => ".bcf",
    }
  }

  pub fn fmt_file(&self, query: &Query) -> String {
    let id = query.id();
    let id = format!("{id}{}", self.file_ending());

    #[cfg(feature = "crypt4gh")]
    if query.object_type().is_crypt4gh() {
      return format!("{id}.c4gh");
    }

    #[allow(clippy::let_and_return)]
    id
  }

  pub fn index_file_ending(&self) -> &str {
    match self {
      Format::Bam => ".bam.bai",
      Format::Cram => ".cram.crai",
      Format::Vcf => ".vcf.gz.tbi",
      Format::Bcf => ".bcf.csi",
    }
  }

  pub fn fmt_index(&self, id: &str) -> String {
    format!("{id}{}", self.index_file_ending())
  }

  pub fn gzi_index_file_ending(&self) -> io::Result<&str> {
    match self {
      Format::Bam => Ok(".bam.gzi"),
      Format::Cram => Err(io::Error::new(
        Other,
        "CRAM does not support GZI".to_string(),
      )),
      Format::Vcf => Ok(".vcf.gz.gzi"),
      Format::Bcf => Ok(".bcf.gzi"),
    }
  }

  pub fn gzi_endings(&self) -> io::Result<&str> {
    match self {
      Format::Bam => Ok(".bam.gzi"),
      Format::Cram => Err(io::Error::new(
        Other,
        "CRAM does not support GZI".to_string(),
      )),
      Format::Vcf => Ok(".vcf.gz.gzi"),
      Format::Bcf => Ok(".bcf.gzi"),
    }
  }

  pub fn fmt_gzi(&self, id: &str) -> io::Result<String> {
    Ok(format!("{id}{}", self.gzi_index_file_ending()?))
  }
}

impl From<Format> for String {
  fn from(format: Format) -> Self {
    format.to_string()
  }
}

impl Display for Format {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Format::Bam => write!(f, "BAM"),
      Format::Cram => write!(f, "CRAM"),
      Format::Vcf => write!(f, "VCF"),
      Format::Bcf => write!(f, "BCF"),
    }
  }
}

/// Class component of htsget response.
#[derive(Copy, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum Class {
  #[serde(alias = "header", alias = "HEADER")]
  Header,
  #[serde(alias = "body", alias = "BODY")]
  Body,
}

/// An interval represents the start (0-based, inclusive) and end (0-based exclusive) ranges of the
/// query.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Interval {
  start: Option<u32>,
  end: Option<u32>,
}

impl Interval {
  /// Check if this interval contains the value.
  pub fn contains(&self, value: u32) -> bool {
    return match (self.start.as_ref(), self.end.as_ref()) {
      (None, None) => true,
      (None, Some(end)) => value < *end,
      (Some(start), None) => value >= *start,
      (Some(start), Some(end)) => value >= *start && value < *end,
    };
  }

  /// Convert this interval into a one-based noodles `Interval`.
  #[instrument(level = "trace", skip_all, ret)]
  pub fn into_one_based(self) -> io::Result<NoodlesInterval> {
    Ok(match (self.start, self.end) {
      (None, None) => NoodlesInterval::from(..),
      (None, Some(end)) => NoodlesInterval::from(..=Self::convert_end(end)?),
      (Some(start), None) => NoodlesInterval::from(Self::convert_start(start)?..),
      (Some(start), Some(end)) => {
        NoodlesInterval::from(Self::convert_start(start)?..=Self::convert_end(end)?)
      }
    })
  }

  /// Convert a start position to a noodles Position.
  pub fn convert_start(start: u32) -> io::Result<Position> {
    Self::convert_position(start, |value| {
      value.checked_add(1).ok_or_else(|| {
        io::Error::new(
          Other,
          format!("could not convert {value} to 1-based position."),
        )
      })
    })
  }

  /// Convert an end position to a noodles Position.
  pub fn convert_end(end: u32) -> io::Result<Position> {
    Self::convert_position(end, Ok)
  }

  /// Convert a u32 position to a noodles Position.
  pub fn convert_position<F>(value: u32, convert_fn: F) -> io::Result<Position>
  where
    F: FnOnce(u32) -> io::Result<u32>,
  {
    let value = convert_fn(value).map(|value| {
      usize::try_from(value)
        .map_err(|err| io::Error::new(Other, format!("could not convert `u32` to `usize`: {err}")))
    })??;

    Position::try_from(value).map_err(|err| {
      io::Error::new(
        Other,
        format!("could not convert `{value}` into `Position`: {err}"),
      )
    })
  }

  pub fn start(&self) -> Option<u32> {
    self.start
  }

  pub fn end(&self) -> Option<u32> {
    self.end
  }

  /// Create a new interval
  pub fn new(start: Option<u32>, end: Option<u32>) -> Self {
    Self { start, end }
  }
}

/// Schemes that can be used with htsget.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Scheme {
  #[default]
  #[serde(alias = "Http", alias = "http")]
  Http,
  #[serde(alias = "Https", alias = "https")]
  Https,
}

impl Display for Scheme {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Scheme::Http => write!(f, "http"),
      Scheme::Https => write!(f, "https"),
    }
  }
}

/// Tagged Any allow type for cors config.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TaggedTypeAll {
  #[serde(alias = "all", alias = "ALL")]
  All,
}

/// Possible values for the fields parameter.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Fields {
  /// Include all fields
  Tagged(TaggedTypeAll),
  /// List of fields to include
  List(HashSet<String>),
}

/// Possible values for the tags parameter.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Tags {
  /// Include all tags
  Tagged(TaggedTypeAll),
  /// List of tags to include
  List(HashSet<String>),
}

/// The no tags parameter.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NoTags(pub Option<HashSet<String>>);

/// A struct containing the information from the HTTP request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
  path: String,
  query: HashMap<String, String>,
  headers: HeaderMap,
}

impl Request {
  /// Create a new request.
  pub fn new(id: String, query: HashMap<String, String>, headers: HeaderMap) -> Self {
    Self {
      path: id,
      query,
      headers,
    }
  }

  /// Create a new request with default query and headers.
  pub fn new_with_id(id: String) -> Self {
    Self::new(id, Default::default(), Default::default())
  }

  /// Set the request headers.
  pub fn with_headers(mut self, headers: HeaderMap) -> Self {
    self.headers = headers;
    self
  }

  /// Get the id.
  pub fn path(&self) -> &str {
    &self.path
  }

  /// Get the query.
  pub fn query(&self) -> &HashMap<String, String> {
    &self.query
  }

  /// Get the headers.
  pub fn headers(&self) -> &HeaderMap {
    &self.headers
  }
}

/// A query contains all the parameters that can be used when requesting
/// a search for either of `reads` or `variants`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
  id: String,
  format: Format,
  class: Class,
  /// Reference name
  reference_name: Option<String>,
  /// The start and end positions are 0-based. [start, end)
  interval: Interval,
  fields: Fields,
  tags: Tags,
  no_tags: NoTags,
  /// The raw HTTP request information.
  request: Request,
  object_type: ObjectType,
}

impl Query {
  /// Create a new query.
  pub fn new(
    id: impl Into<String>,
    format: Format,
    request: Request,
    object_type: ObjectType,
  ) -> Self {
    Self {
      id: id.into(),
      format,
      class: Class::Body,
      reference_name: None,
      interval: Interval::default(),
      fields: Fields::Tagged(TaggedTypeAll::All),
      tags: Tags::Tagged(TaggedTypeAll::All),
      no_tags: NoTags(None),
      request,
      object_type,
    }
  }

  /// Create a new query with a default request.
  pub fn new_with_defaults(id: impl Into<String>, format: Format) -> Self {
    let id = id.into();
    Self::new(
      id.clone(),
      format,
      Request::new_with_id(id),
      Default::default(),
    )
  }

  /// Set the id.
  pub fn set_id(&mut self, id: impl Into<String>) {
    self.id = id.into();
  }

  /// Set the is and return self.
  pub fn with_id(mut self, id: impl Into<String>) -> Self {
    self.set_id(id);
    self
  }

  /// Set the format.
  pub fn with_format(mut self, format: Format) -> Self {
    self.format = format;
    self
  }

  /// Set the class.
  pub fn with_class(mut self, class: Class) -> Self {
    self.class = class;
    self
  }

  /// Set the reference name.
  pub fn with_reference_name(mut self, reference_name: impl Into<String>) -> Self {
    self.reference_name = Some(reference_name.into());
    self
  }

  /// Set the interval.
  pub fn with_start(mut self, start: u32) -> Self {
    self.interval.start = Some(start);
    self
  }

  /// Set the interval.
  pub fn with_end(mut self, end: u32) -> Self {
    self.interval.end = Some(end);
    self
  }

  /// Set the interval.
  pub fn with_fields(mut self, fields: Fields) -> Self {
    self.fields = fields;
    self
  }

  /// Set the interval.
  pub fn with_tags(mut self, tags: Tags) -> Self {
    self.tags = tags;
    self
  }

  /// Set no tags.
  pub fn with_no_tags(mut self, no_tags: Vec<impl Into<String>>) -> Self {
    self.no_tags = NoTags(Some(
      no_tags.into_iter().map(|field| field.into()).collect(),
    ));
    self
  }

  pub fn id(&self) -> &str {
    &self.id
  }

  pub fn format(&self) -> Format {
    self.format
  }

  pub fn class(&self) -> Class {
    self.class
  }

  pub fn reference_name(&self) -> Option<&str> {
    self.reference_name.as_deref()
  }

  pub fn interval(&self) -> Interval {
    self.interval
  }

  pub fn fields(&self) -> &Fields {
    &self.fields
  }

  pub fn tags(&self) -> &Tags {
    &self.tags
  }

  pub fn no_tags(&self) -> &NoTags {
    &self.no_tags
  }

  pub fn request(&self) -> &Request {
    &self.request
  }

  /// Get the object type of this query.
  pub fn object_type(&self) -> &ObjectType {
    &self.object_type
  }

  /// Set the object type.
  pub fn with_object_type(mut self, object_type: ObjectType) -> Self {
    self.set_object_type(object_type);
    self
  }

  /// Set the object type.
  pub fn set_object_type(&mut self, object_type: ObjectType) {
    self.object_type = object_type;
  }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum HtsGetError {
  #[error("not found: {0}")]
  NotFound(String),

  #[error("unsupported Format: {0}")]
  UnsupportedFormat(String),

  #[error("invalid input: {0}")]
  InvalidInput(String),

  #[error("invalid range: {0}")]
  InvalidRange(String),

  #[error("io error: {0}")]
  IoError(String),

  #[error("parsing error: {0}")]
  ParseError(String),

  #[error("internal error: {0}")]
  InternalError(String),
}

impl HtsGetError {
  pub fn not_found<S: Into<String>>(message: S) -> Self {
    Self::NotFound(message.into())
  }

  pub fn unsupported_format<S: Into<String>>(format: S) -> Self {
    Self::UnsupportedFormat(format.into())
  }

  pub fn invalid_input<S: Into<String>>(message: S) -> Self {
    Self::InvalidInput(message.into())
  }

  pub fn invalid_range<S: Into<String>>(message: S) -> Self {
    Self::InvalidRange(message.into())
  }

  pub fn io_error<S: Into<String>>(message: S) -> Self {
    Self::IoError(message.into())
  }

  pub fn parse_error<S: Into<String>>(message: S) -> Self {
    Self::ParseError(message.into())
  }

  pub fn internal_error<S: Into<String>>(message: S) -> Self {
    Self::InternalError(message.into())
  }
}

impl From<HtsGetError> for io::Error {
  fn from(error: HtsGetError) -> Self {
    Self::new(Other, error)
  }
}

impl From<io::Error> for HtsGetError {
  fn from(err: io::Error) -> Self {
    Self::io_error(err.to_string())
  }
}

/// The headers that need to be supplied when requesting data from a url.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Headers(BTreeMap<String, String>);

impl Headers {
  pub fn new(headers: BTreeMap<String, String>) -> Self {
    Self(headers)
  }

  /// Insert an entry into the headers. If the entry already exists, the value will be appended to
  /// the existing value, separated by a comma. Returns self.
  pub fn with_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
    self.insert(key, value);
    self
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Insert an entry into the headers. If the entry already exists, the value will be appended to
  /// the existing value, separated by a comma.
  pub fn insert<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
    let entry = self.0.entry(key.into()).or_default();
    if entry.is_empty() {
      entry.push_str(&value.into());
    } else {
      entry.push_str(&format!(", {}", value.into()));
    }
  }

  /// Add to the headers.
  pub fn extend(&mut self, headers: Headers) {
    self.0.extend(headers.into_inner());
  }

  /// Get the inner BTreeMap.
  pub fn into_inner(self) -> BTreeMap<String, String> {
    self.0
  }

  /// Get a reference to the inner BTreeMap.
  pub fn as_ref_inner(&self) -> &BTreeMap<String, String> {
    &self.0
  }
}

impl TryFrom<&HeaderMap> for Headers {
  type Error = Error;

  fn try_from(headers: &HeaderMap) -> result::Result<Self, Self::Error> {
    headers
      .iter()
      .try_fold(Headers::default(), |acc, (key, value)| {
        Ok(acc.with_header(
          key.to_string(),
          value.to_str().map_err(|err| {
            ParseError(format!("failed to convert header value to string: {}", err))
          })?,
        ))
      })
  }
}

/// A url from which raw data can be retrieved.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Url {
  pub url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub headers: Option<Headers>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub class: Option<Class>,
}

impl Url {
  /// Create a new Url.
  pub fn new<S: Into<String>>(url: S) -> Self {
    Self {
      url: url.into(),
      headers: None,
      class: None,
    }
  }

  /// Add to the headers of the Url.
  pub fn add_headers(mut self, headers: Headers) -> Self {
    if !headers.is_empty() {
      self
        .headers
        .get_or_insert_with(Headers::default)
        .extend(headers);
    }

    self
  }

  /// Set the headers of the Url.
  pub fn with_headers(mut self, headers: Headers) -> Self {
    self.headers = Some(headers).filter(|h| !h.is_empty());
    self
  }

  /// Set the class of the Url using an optional value.
  pub fn set_class(mut self, class: Option<Class>) -> Self {
    self.class = class;
    self
  }

  /// Set the class of the Url.
  pub fn with_class(self, class: Class) -> Self {
    self.set_class(Some(class))
  }
}

/// Wrapped json response for htsget.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonResponse {
  pub htsget: Response,
}

impl JsonResponse {
  pub fn new(htsget: Response) -> Self {
    Self { htsget }
  }
}

impl From<Response> for JsonResponse {
  fn from(htsget: Response) -> Self {
    Self::new(htsget)
  }
}

/// The response for a HtsGet query.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
  pub format: Format,
  pub urls: Vec<Url>,
}

impl Response {
  pub fn new(format: Format, urls: Vec<Url>) -> Self {
    Self { format, urls }
  }
}

#[cfg(test)]
mod tests {
  use std::collections::{BTreeMap, HashSet};
  use std::str::FromStr;

  use http::{HeaderMap, HeaderName, HeaderValue};
  use serde_json::{json, to_value};

  use crate::types::{
    Class, Fields, Format, Headers, HtsGetError, Interval, NoTags, Query, Response, TaggedTypeAll,
    Tags, Url,
  };

  #[test]
  fn interval_contains() {
    let interval = Interval {
      start: Some(0),
      end: Some(10),
    };
    assert!(interval.contains(9));
  }

  #[test]
  fn interval_not_contains() {
    let interval = Interval {
      start: Some(0),
      end: Some(10),
    };
    assert!(!interval.contains(10));
  }

  #[test]
  fn interval_contains_start_not_present() {
    let interval = Interval {
      start: None,
      end: Some(10),
    };
    assert!(interval.contains(9));
  }

  #[test]
  fn interval_not_contains_start_not_present() {
    let interval = Interval {
      start: None,
      end: Some(10),
    };
    assert!(!interval.contains(10));
  }

  #[test]
  fn interval_contains_end_not_present() {
    let interval = Interval {
      start: Some(1),
      end: None,
    };
    assert!(interval.contains(9));
  }

  #[test]
  fn interval_not_contains_end_not_present() {
    let interval = Interval {
      start: Some(1),
      end: None,
    };
    assert!(!interval.contains(0));
  }

  #[test]
  fn interval_contains_both_not_present() {
    let interval = Interval {
      start: None,
      end: None,
    };
    assert!(interval.contains(0));
  }

  #[test]
  fn htsget_error_not_found() {
    let result = HtsGetError::not_found("error");
    assert!(matches!(result, HtsGetError::NotFound(message) if message == "error"));
  }

  #[test]
  fn htsget_error_unsupported_format() {
    let result = HtsGetError::unsupported_format("error");
    assert!(matches!(result, HtsGetError::UnsupportedFormat(message) if message == "error"));
  }

  #[test]
  fn htsget_error_invalid_input() {
    let result = HtsGetError::invalid_input("error");
    assert!(matches!(result, HtsGetError::InvalidInput(message) if message == "error"));
  }

  #[test]
  fn htsget_error_invalid_range() {
    let result = HtsGetError::invalid_range("error");
    assert!(matches!(result, HtsGetError::InvalidRange(message) if message == "error"));
  }

  #[test]
  fn htsget_error_io_error() {
    let result = HtsGetError::io_error("error");
    assert!(matches!(result, HtsGetError::IoError(message) if message == "error"));
  }

  #[test]
  fn htsget_error_parse_error() {
    let result = HtsGetError::parse_error("error");
    assert!(matches!(result, HtsGetError::ParseError(message) if message == "error"));
  }

  #[test]
  fn htsget_error_internal_error() {
    let result = HtsGetError::internal_error("error");
    assert!(matches!(result, HtsGetError::InternalError(message) if message == "error"));
  }

  #[test]
  fn query_new() {
    let result = Query::new_with_defaults("NA12878", Format::Bam);
    assert_eq!(result.id(), "NA12878");
  }

  #[test]
  fn query_with_format() {
    let result = Query::new_with_defaults("NA12878", Format::Bam);
    assert_eq!(result.format(), Format::Bam);
  }

  #[test]
  fn query_with_class() {
    let result = Query::new_with_defaults("NA12878", Format::Bam).with_class(Class::Header);
    assert_eq!(result.class(), Class::Header);
  }

  #[test]
  fn query_with_reference_name() {
    let result = Query::new_with_defaults("NA12878", Format::Bam).with_reference_name("chr1");
    assert_eq!(result.reference_name(), Some("chr1"));
  }

  #[test]
  fn query_with_start() {
    let result = Query::new_with_defaults("NA12878", Format::Bam).with_start(0);
    assert_eq!(result.interval().start(), Some(0));
  }

  #[test]
  fn query_with_end() {
    let result = Query::new_with_defaults("NA12878", Format::Bam).with_end(0);
    assert_eq!(result.interval().end(), Some(0));
  }

  #[test]
  fn query_with_fields() {
    let result = Query::new_with_defaults("NA12878", Format::Bam).with_fields(Fields::List(
      HashSet::from_iter(vec!["QNAME".to_string(), "FLAG".to_string()]),
    ));
    assert_eq!(
      result.fields(),
      &Fields::List(HashSet::from_iter(vec![
        "QNAME".to_string(),
        "FLAG".to_string()
      ]))
    );
  }

  #[test]
  fn query_with_tags() {
    let result =
      Query::new_with_defaults("NA12878", Format::Bam).with_tags(Tags::Tagged(TaggedTypeAll::All));
    assert_eq!(result.tags(), &Tags::Tagged(TaggedTypeAll::All));
  }

  #[test]
  fn query_with_no_tags() {
    let result = Query::new_with_defaults("NA12878", Format::Bam).with_no_tags(vec!["RG", "OQ"]);
    assert_eq!(
      result.no_tags(),
      &NoTags(Some(HashSet::from_iter(vec![
        "RG".to_string(),
        "OQ".to_string()
      ])))
    );
  }

  #[test]
  fn format_from_bam() {
    let result = String::from(Format::Bam);
    assert_eq!(result, "BAM");
  }

  #[test]
  fn format_from_cram() {
    let result = String::from(Format::Cram);
    assert_eq!(result, "CRAM");
  }

  #[test]
  fn format_from_vcf() {
    let result = String::from(Format::Vcf);
    assert_eq!(result, "VCF");
  }

  #[test]
  fn format_from_bcf() {
    let result = String::from(Format::Bcf);
    assert_eq!(result, "BCF");
  }

  #[test]
  fn headers_with_header() {
    let header = Headers::new(BTreeMap::new()).with_header("Range", "bytes=0-1023");
    let result = header.0.get("Range");
    assert_eq!(result, Some(&"bytes=0-1023".to_string()));
  }

  #[test]
  fn headers_is_empty() {
    assert!(Headers::new(BTreeMap::new()).is_empty());
  }

  #[test]
  fn headers_insert() {
    let mut header = Headers::new(BTreeMap::new());
    header.insert("Range", "bytes=0-1023");
    let result = header.0.get("Range");
    assert_eq!(result, Some(&"bytes=0-1023".to_string()));
  }

  #[test]
  fn headers_extend() {
    let mut headers = Headers::new(BTreeMap::new());
    headers.insert("Range", "bytes=0-1023");

    let mut extend_with = Headers::new(BTreeMap::new());
    extend_with.insert("header", "value");

    headers.extend(extend_with);

    let result = headers.0.get("Range");
    assert_eq!(result, Some(&"bytes=0-1023".to_string()));

    let result = headers.0.get("header");
    assert_eq!(result, Some(&"value".to_string()));
  }

  #[test]
  fn headers_multiple_values() {
    let headers = Headers::new(BTreeMap::new())
      .with_header("Range", "bytes=0-1023")
      .with_header("Range", "bytes=1024-2047");
    let result = headers.0.get("Range");

    assert_eq!(result, Some(&"bytes=0-1023, bytes=1024-2047".to_string()));
  }

  #[test]
  fn headers_try_from_header_map() {
    let mut headers = HeaderMap::new();
    headers.append(
      HeaderName::from_str("Range").unwrap(),
      HeaderValue::from_str("bytes=0-1023").unwrap(),
    );
    headers.append(
      HeaderName::from_str("Range").unwrap(),
      HeaderValue::from_str("bytes=1024-2047").unwrap(),
    );
    headers.append(
      HeaderName::from_str("Range").unwrap(),
      HeaderValue::from_str("bytes=2048-3071, bytes=3072-4095").unwrap(),
    );
    let headers: Headers = (&headers).try_into().unwrap();

    let result = headers.0.get("range");
    assert_eq!(
      result,
      Some(&"bytes=0-1023, bytes=1024-2047, bytes=2048-3071, bytes=3072-4095".to_string())
    );
  }

  #[test]
  fn serialize_headers() {
    let headers = Headers::new(BTreeMap::new())
      .with_header("Range", "bytes=0-1023")
      .with_header("Range", "bytes=1024-2047");

    let result = to_value(headers).unwrap();
    assert_eq!(
      result,
      json!({
        "Range" : "bytes=0-1023, bytes=1024-2047"
      })
    );
  }

  #[test]
  fn url_with_headers() {
    let result = Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==")
      .with_headers(Headers::new(BTreeMap::new()));
    assert_eq!(result.headers, None);
  }

  #[test]
  fn url_add_headers() {
    let mut headers = Headers::new(BTreeMap::new());
    headers.insert("Range", "bytes=0-1023");

    let mut extend_with = Headers::new(BTreeMap::new());
    extend_with.insert("header", "value");

    let result = Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==")
      .with_headers(headers)
      .add_headers(extend_with);

    let expected_headers = Headers::new(BTreeMap::new())
      .with_header("Range", "bytes=0-1023")
      .with_header("header", "value");

    assert_eq!(result.headers, Some(expected_headers));
  }

  #[test]
  fn url_with_class() {
    let result =
      Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==").with_class(Class::Header);
    assert_eq!(result.class, Some(Class::Header));
  }

  #[test]
  fn url_set_class() {
    let result =
      Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==").set_class(Some(Class::Header));
    assert_eq!(result.class, Some(Class::Header));
  }

  #[test]
  fn url_new() {
    let result = Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==");
    assert_eq!(result.url, "data:application/vnd.ga4gh.bam;base64,QkFNAQ==");
    assert_eq!(result.headers, None);
    assert_eq!(result.class, None);
  }

  #[test]
  fn response_new() {
    let result = Response::new(
      Format::Bam,
      vec![Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==")],
    );
    assert_eq!(result.format, Format::Bam);
    assert_eq!(
      result.urls,
      vec![Url::new("data:application/vnd.ga4gh.bam;base64,QkFNAQ==")]
    );
  }
}
