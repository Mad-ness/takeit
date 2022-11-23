use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path,
    io::Error as StdIoError,
    io::Read,
    convert::TryFrom,
    collections::{HashSet, HashMap},
    time::{Duration, Instant},
};


pub type ParamValue = serde_json::Value;

// DOCUMENT VALUE //

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DocumentValueType {
    Array,
    Boolean,
    Hash,
    Number,
    Json,
    Yaml,
    #[default]
    String,
}

// OVERRIDE //

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Override {
    pub omit: bool,
    pub value: ParamValue,
    #[serde(rename="match", deserialize_with = "str2attr")]
    pub attrs: HashMap<String, String>,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OverrideV2 {
    pub omit: bool,
    pub value: ParamValue,
}

pub type DocumentOverrides = HashMap<String, OverrideV2>;

/*****************************
    DOCUMENT VERSION 2
*****************************/
#[derive(Clone, Deserialize)]
pub struct Document {
    pub description: String,
    pub default_value: ParamValue,
    #[serde(rename = "override")]
    pub enabled: bool,
    #[serde(rename = "parameter_type")]
    pub value_type: DocumentValueType,
    #[serde(rename = "parameter")]
    pub name: String,
    #[serde(rename = "puppetclass_name")]
    pub collection: String,
    omit: bool,
    pub merge_default: bool,
    pub merge_overrides: bool,
    //pub overrides: Option<Vec<OverrideV2>>,
    #[serde(rename = "override_values", deserialize_with = "deser_overrides")]
    pub overrides: DocumentOverrides,
    #[serde(rename = "override_value_order", deserialize_with = "str2list_of_attrs")]
    pub order_list: Vec<Vec<String>>,

    pub hidden_value: Option<bool>,
    pub validator_rule: Option<String>,
    pub validator_type: Option<String>,

    /*** Extra attributes for ease management ***/
    // #[serde(skip)]
    // pub attr_list: Vec<String>, // a list of attributes required to lookup value
}


impl Document {
    pub fn total_overrides(&self) -> usize {
        self.overrides.len()
    }

    pub fn override_attrs(&self) -> Vec<String> {
        let mut items: HashSet<String> = HashSet::default();
        for row_items in self.order_list.iter() {
            row_items.iter().for_each(|it| {
                items.insert(it.to_lowercase().into());
            })
        }
        items.iter().map(|it| it.clone()).collect::<Vec<String>>()
    }

    pub fn override_order(&self) -> Vec<String> {
        self.order_list.iter().map(|attrs| attrs.join(",")).collect::<Vec<String>>()
    }

    ///
    /// Loog up a value from the document for given attributes
    ///
    pub fn get_value(&self, attrs: &HashMap<String, String>) -> ParamValue {
        let is_hash = [
            DocumentValueType::Hash,
            DocumentValueType::Json,
            DocumentValueType::Yaml
        ].contains(&self.value_type);
        let mut value: ParamValue = match self.merge_default && is_hash {
            true => self.default_value.clone(),
            false => serde_json::json!({}),
        };
        let need_merge: bool = self.merge_overrides && is_hash;
        for order_key in &self.order_list {
            let override_key = build_compare_key(&attrs, &order_key, true);
            match self.overrides.get(&override_key) {
                Some(ref matcher) => match need_merge {
                    true => if ! matcher.omit { json_patch::merge(&mut value, &matcher.value) },
                    false =>  { value = matcher.value.clone(); break; }
                }
                None => ()
            }
        }
        match value == serde_json::json!({}) {
            true => self.default_value.clone(),
            false => value,
        }
    }

    pub fn get_overrides(&self) -> DocumentOverrides {
        self.overrides.clone()
    }
}

impl TryFrom<&path::Path> for Document {
    type Error = DocumentError;

    fn try_from(path: &path::Path) -> Result<Self, Self::Error> {
        let mut content = String::new();
        std::fs::File::open(path)?.read_to_string(&mut content)?;
        Ok(Document::try_from(content.as_str())?)
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({} overrides)", &self.name, self.overrides.len())
    }
}

impl TryFrom<&str> for Document {
    type Error = DocumentError;
    fn try_from(buffer: &str) -> Result<Self, Self::Error> {
        let start = Instant::now();
        let mut item: Document = serde_yaml::from_str(buffer)?;
        item.name = item.name.to_lowercase();
        item.collection = item.collection.to_lowercase();
        tracing::info!("loaded document {}/{} in {:?}", &item.collection, &item.name, &start.elapsed());
        Ok(item)
    }
}

///
/// Deserialize a list of matchers into a hashmap of matchers
///
/// A list of such elements
/// ```not_run
/// - key: key1=value1,key2=value
///   omit: false
///   value: "Hello, World"
/// ```
/// is deserialized into hashmap of these elements
/// ```not_run
/// key1=value1,key2=value:
///     omit: false
///     value: "Hello, World"
/// ```
///
fn deser_overrides<'de, D>(deserializer: D) -> std::result::Result<DocumentOverrides, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Matcher {
        pub omit: bool,
        pub value: ParamValue,
        #[serde(rename="match", deserialize_with = "match_to_string")]
        pub key: String,
    }
    let items: Vec<Matcher> = Deserialize::deserialize(deserializer)?;
    let items = items.into_iter()
        .map(|it| (it.key.clone(), OverrideV2 { omit: it.omit, value: it.value.clone() }))
        .collect::<DocumentOverrides>();
    Ok(items)
}



/*
Split up a matcher key given as string into key value pairs.
Ex., domain=example.com,is_virtual=true => {domain: example.com, is_virtual: true}
*/
fn str2attr<'de, D>(deserializer: D) -> std::result::Result<HashMap<String, String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let item: &str = Deserialize::deserialize(deserializer)?;
    let mut result: HashMap<String, String> = HashMap::new();
    for pair in item.split_terminator(",").collect::<Vec<&str>>() {
        let (attr, value) = pair.split_once('=')
            .ok_or_else(||serde::de::Error::custom(String::from(format!("matcher {}, attr {}", &item, &pair))))?;
        result.insert(attr.to_lowercase().into(), value.into());
    }
    Ok(result)
}

fn match_to_string<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let item: &str = Deserialize::deserialize(deserializer)?;
    Ok(normalize_override_key(item).into())
}

///
/// Get a list of attributes required to lookup value.
/// It converts a list of list of attributes
///  - `[fqdn]`
///  - `[domain]`
///  - `[hostgroup,is_virtual,domain]`
/// into a hashset
///  - `fqdn`
///  - `domain`
///  - `hostgroup`
///  - `is_virtual`
///
fn extract_attrs(list_attrs: &Vec<Vec<String>>) -> Vec<String> {
    let mut req_attrs: Vec<String> = Vec::new();
    list_attrs.iter().for_each(|list| {
        list.iter().for_each(|it| {
            req_attrs.push(it.clone());
        });
    });
    req_attrs.sort();
    req_attrs.dedup();
    req_attrs
}

#[derive(Debug)]
pub enum DocumentError {
    StdIoError(std::io::Error),
    ParseError(serde_yaml::Error),
    ContentError(String),
}

impl From<String> for DocumentError {
    fn from(inner: String) -> Self {
        DocumentError::ContentError(inner)
    }
}

impl From<serde_yaml::Error> for DocumentError {
    fn from(inner: serde_yaml::Error) -> Self {
        DocumentError::ParseError(inner)
    }
}

impl From<std::io::Error> for DocumentError {
    fn from(inner: std::io::Error) -> Self {
        DocumentError::StdIoError(inner)
    }
}

pub struct DocumentInfo {
    name: String,
    collection_name: String,
    overrides_enabled: bool,
    total_overrides: usize,
    order_list: Vec<String>,
    override_attrs: Vec<String>,
    value_type: DocumentValueType,
    default_value: ParamValue,
}

impl From<&Document> for DocumentInfo {
    fn from(doc: &Document) -> Self {
        Self {
            name: doc.name.clone(),
            collection_name: doc.collection.clone(),
            overrides_enabled: doc.enabled,
            total_overrides: doc.overrides.len(),
            override_attrs: doc.override_attrs().iter().map(|it| it.clone()).collect(),
            order_list: doc.order_list.iter().map(|it| it.join(",")).collect(),
            value_type: doc.value_type.clone(),
            default_value: doc.default_value.clone(),
        }
    }
}

///
/// Normalize an override match attribute.
/// All override match keys will be represented in lowercase and sorted order.
///
/// # Examples
/// - `is_virtual=TruE,HostGroup=Dev` => `hostgroup=dev,is_virtual=true`
/// - `is_virtual  = fALSe,  hostgroup  = PROD` => `hostgroup=prod,is_virtual=false`
///
pub fn normalize_override_key(value: &str) -> String {
    let ensure = |s: &str| {
        s.split_once('=')
            .map_or_else(
                |            | s.trim().into(),
                |(key, value)| format!("{}={}", &key.trim(), &value.trim())
            ).to_lowercase()
    };
    let mut value: Vec<String> = value.split_terminator(',')
        .map(|keyvalue| ensure(&keyvalue))
        .collect();
    value.sort();
    value.join(",")
}

///
/// Normalize `key=value` attributes into a string.
///
/// It works like `normalize_override_key` but only accepts input as a `HashMap`.
///
/// # Arguments
/// - `attrs` a `HashMap` of `key=value` pairs
/// - `to_lowercase` if `true` then value will be lowercased. Key is always lowercased
///
/// # Return a sorted comma separated string of the attributes.
/// Only key names are used for sorting:
/// `key_A=value1,Key_Z=value2,KEY_d=value3` => `key_a=value1,key_d=value3,key_z=value2`
///
fn normalize_attrs(attrs: &HashMap<String, String>, to_lowercase: bool) -> String {
    let mut pairs: Vec<String> = attrs.iter().map(|(key, value)| {
        format!(
            "{}={}",
            &key.trim().to_lowercase(),
            {
                let value = String::from(value.trim());
                match to_lowercase {
                    true => value.to_lowercase(),
                    false => value,
                }
            }
        )
    }).collect();
    pairs.sort();
    pairs.join(",")
}

///
/// The function calls `normalize_attrs` and work with only attributes provided in `list_attrs`.
/// Attributes not found in `attrs` get empty values
///
fn build_compare_key(attrs: &HashMap<String, String>, list_attrs: &Vec<String>, to_lowercase: bool) -> String {
    // the function is not optimal - too many clones
    assert_eq!(list_attrs.len() > 0, true);
    let attrs = list_attrs.into_iter()
        .map(|it| (it.to_lowercase().clone(), attrs.get(it).unwrap_or(&"".into()).clone()))
        .collect::<HashMap<String, String>>();
    normalize_attrs(&attrs, to_lowercase)
}

/*
Parse order list into list of list of attributes
Ex.,
- fqdn
- domain,is_virtual
- hostgroup,is_virtual

becomes [[fqdn], [domain,is_virtual], [hostgroup,is_virtual]]
*/
fn str2list_of_attrs<'de, D>(deserializer: D) -> std::result::Result<Vec<Vec<String>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let list_attrs: Vec<&str> = Deserialize::deserialize(deserializer)?;
    let mut result = Vec::<Vec<String>>::new();
    for it in list_attrs.iter() {
        let mut attrs = Vec::<String>::new();
        for i in it.split_terminator(",").collect::<Vec<&str>>().iter() {
            attrs.push(i.to_string().to_lowercase());
        };
        attrs.sort();
        attrs.dedup();
        result.push(attrs);
    };
    Ok(result)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use crate::documentv2::{
        Document, normalize_override_key, normalize_attrs,
        build_compare_key
    };

    const DOC1_YAML: &str = r#"
    description: Test document
    default_value: "Hello, World"
    override: true
    parameter_type: string
    parameter: hello
    puppetclass_name: world
    omit: false
    merge_default: false
    merge_overrides: false
    override_values:
      - match: key1=value1,key2=value2
        omit: false
        value: Hello, key1, key2
      - match: key2=value2,key3=value3
        omit: false
        value: Hello, key2, key3
    override_value_order:
      - key1,key2
      - key2,key3
    hidden_value: false
    validator_rule: null
    validator_type: null
    "#;
    #[test]
    fn test_doc_v2() {
        let doc = Document::try_from(DOC1_YAML).expect("could not parse document");
        assert_eq!(doc.total_overrides(), 2);
        assert_eq!(doc.default_value, String::from("Hello, World"));
        assert_eq!(doc.get_value(
            &HashMap::<String, String>::from([
                ("key2".into(), "value2".into()),
                ("key3".into(), "value3".into()),
            ])
        ), "Hello, key2, key3");
    }

    #[test]
    fn test_normalize_override_key() {
        let tests: Vec<(&str, &str)> = vec![
            ( "is_virtual=TruE,HostGroup=Dev", "hostgroup=dev,is_virtual=true" ),
            ( "domain=example.com ,  HostGroup=pRODd", "domain=example.com,hostgroup=prodd" ),
            ( "ad-group=true,domain=example.com , HostGroup=pRODd", "ad-group=true,domain=example.com,hostgroup=prodd" ),
            ( " HostGroup=pRODd,any=", "any=,hostgroup=prodd" ),
            ( " all,HostGroup=pRODd,any=", "all,any=,hostgroup=prodd" ),
            ( " all=NO,HostGroup=pRODd, any= ,ZED=always ", "all=no,any=,hostgroup=prodd,zed=always" ),
            ( " HostGroup=pRO d, any  =  AAA BBB ", "any=aaa bbb,hostgroup=pro d" ),
        ];
        for (s1, s2) in tests {
            assert_eq!(s2, normalize_override_key(s1));
        }
    }

    #[test]
    fn test_deser_overrides() {
        let source: &str = r#"
        - key: key1=value1,key2=value2
          omit: false
          value: Hello, World
        - key: key3=value1,key3=value2
          omit: false
          value: Hello, everyone
        "#;
    }

    #[test]
    fn test_normalize_attrs() {
        let attrs: HashMap<String, String> = HashMap::from([
            ("KEY_a ".into(), "value_1  ".into()),
            ("kEY_z".into(), "   VALUE_2".into()),
            ("  key_D".into(), "valUE_3  ".into()),
        ]);
        let r1 = normalize_attrs(&attrs, false);
        assert_eq!(&r1, "key_a=value_1,key_d=valUE_3,key_z=VALUE_2");
        let r1 = normalize_attrs(&attrs, true);
        assert_eq!(&r1, "key_a=value_1,key_d=value_3,key_z=value_2");

        let attrs: HashMap<String, String> = HashMap::from([
            ("key_a".into(), "value_1  ".into()),
            ("key_z".into(), "   VALUE_2".into()),
            ("key_d".into(), "valUE_3  ".into()),
        ]);
        let r1 = build_compare_key(&attrs, &vec!["key_a".into(), "key_z".into(), "a_key".into()], true);
        assert_eq!(&r1, "a_key=,key_a=value_1,key_z=value_2");
    }
}
