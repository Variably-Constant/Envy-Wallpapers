//! The catalog that lists every wallpaper the store offers: what each manifest says of it, where its package and thumbnail
//! are, and the package's size and SHA-256, so the TV can tell a package it downloads is the one listed. The store's
//! repository signs `catalog.json` with minisign, and [`verify`] checks that signature against the store's public key.

use serde::{Deserialize, Serialize};

use crate::manifest::{Holiday, Kind, License, Manifest, Play, Season, FORMAT};

/// Every wallpaper the store offers, in the order of their ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// The format of the wallpapers it lists, [`FORMAT`].
    pub format: u32,
    pub wallpapers: Vec<Entry>,
}

/// One wallpaper in the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub kind: Kind,
    pub author: String,
    pub license: License,
    pub description: String,
    pub tags: Vec<String>,
    pub plays: Vec<Play>,
    pub season: Option<Season>,
    pub holiday: Option<Holiday>,
    /// Whether it carries a script.
    pub script: bool,
    /// The package's size in bytes.
    pub size: u64,
    /// The package's SHA-256, in lowercase hexadecimal.
    pub sha256: String,
    /// Where the package downloads from.
    pub package: String,
    /// Where the thumbnail downloads from.
    pub thumbnail: String,
}

impl Entry {
    /// The entry for `manifest`, whose package of `size` bytes with `sha256` is at `package`, and its thumbnail at
    /// `thumbnail`.
    pub fn new(manifest: &Manifest, size: u64, sha256: String, package: String, thumbnail: String) -> Self {
        Entry {
            id: manifest.id.clone(),
            title: manifest.title.clone(),
            version: manifest.version.clone(),
            kind: manifest.kind,
            author: manifest.author.clone(),
            license: manifest.license,
            description: manifest.description.clone(),
            tags: manifest.tags.clone(),
            plays: manifest.plays.clone(),
            season: manifest.season,
            holiday: manifest.holiday,
            script: manifest.script.is_some(),
            size,
            sha256,
            package,
            thumbnail,
        }
    }
}

impl Catalog {
    /// The catalog of `wallpapers`, put in the order of their ids.
    pub fn new(mut wallpapers: Vec<Entry>) -> Self {
        wallpapers.sort_by(|a, b| a.id.cmp(&b.id));
        Catalog { format: FORMAT, wallpapers }
    }

    /// The catalog as `catalog.json` holds it.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// The catalog `text` holds, if it is one in a format this crate reads.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let catalog: Catalog = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if catalog.format != FORMAT {
            return Err(format!("a catalog of format {} where this reads {FORMAT}", catalog.format));
        }
        Ok(catalog)
    }
}

/// Checks that `signature`, the text of a minisign signature file, signs `catalog` under `public_key`, a minisign public
/// key in base64.
pub fn verify(catalog: &[u8], signature: &str, public_key: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::from_base64(public_key).map_err(|e| format!("the public key: {e}"))?;
    let signed = minisign_verify::Signature::decode(signature).map_err(|e| format!("the signature: {e}"))?;
    key.verify(catalog, &signed, false).map_err(|e| format!("the catalog's signature: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> Entry {
        let text = crate::manifest::tests::SCENE.replace("\"leaf-pile\"", &format!("\"{id}\""));
        let manifest = Manifest::parse(&text).expect("the scene reads");
        Entry::new(&manifest, 1234, "ab".repeat(32), format!("https://example.com/{id}.tar"), format!("https://example.com/{id}/thumb.png"))
    }

    #[test]
    fn an_entry_carries_what_the_manifest_says_and_where_its_package_is() {
        let entry = entry("leaf-pile");
        assert_eq!(entry.title, "Leaf Pile");
        assert_eq!(entry.kind, Kind::Scene);
        assert_eq!(entry.license, License::CcBySa);
        assert_eq!(entry.season, Some(Season::Autumn));
        assert!(!entry.script);
        assert_eq!(entry.size, 1234);
        assert_eq!(entry.package, "https://example.com/leaf-pile.tar");
    }

    #[test]
    fn a_catalog_lists_by_id_and_reads_back_from_its_json() {
        let catalog = Catalog::new(vec![entry("snow-day"), entry("leaf-pile")]);
        assert_eq!(catalog.wallpapers[0].id, "leaf-pile");
        let json = catalog.to_json().expect("the catalog's json");
        assert!(json.contains("\"license\": \"CC-BY-SA-4.0\""), "{json}");
        assert!(json.contains("\"plays\": [\n        \"pointer-wind\""), "{json}");
        assert_eq!(Catalog::from_json(&json).expect("it reads back"), catalog);
        let newer = json.replacen("\"format\": 1", "\"format\": 2", 1);
        assert!(Catalog::from_json(&newer).is_err());
    }

    #[test]
    fn the_stores_signature_verifies_and_a_changed_catalog_does_not() {
        let keys = minisign::KeyPair::generate_unencrypted_keypair().expect("a key pair");
        let json = Catalog::new(vec![entry("leaf-pile")]).to_json().expect("the catalog's json");
        let signature = minisign::sign(Some(&keys.pk), &keys.sk, std::io::Cursor::new(json.as_bytes()), Some("catalog.json"), None).expect("a signature");
        let public_key = keys.pk.to_base64();
        verify(json.as_bytes(), &signature.to_string(), &public_key).expect("the store's signature verifies");
        let changed = json.replace("1234", "1235");
        assert!(verify(changed.as_bytes(), &signature.to_string(), &public_key).is_err());
        let other = minisign::KeyPair::generate_unencrypted_keypair().expect("a key pair");
        assert!(verify(json.as_bytes(), &signature.to_string(), &other.pk.to_base64()).is_err());
    }
}
