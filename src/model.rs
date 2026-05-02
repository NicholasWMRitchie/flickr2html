use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AlbumsFile {
    pub albums: Vec<Album>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Album {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub photo_count: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub photos: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Photo {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub date_taken: String,
    #[serde(default)]
    pub rotation: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_albums_fixture() {
        let json = r#"{
            "albums": [
                {
                    "photo_count": "2",
                    "id": "1001",
                    "url": "https://example/x",
                    "title": "Trip",
                    "description": "A trip.",
                    "view_count": "1",
                    "created": "1700000000",
                    "last_updated": "1700000001",
                    "cover_photo": "https://example/0",
                    "photos": ["a","b"]
                },
                {
                    "photo_count": "0",
                    "id": "1002",
                    "url": "https://example/y",
                    "title": "Empty",
                    "description": "",
                    "view_count": "0",
                    "created": "1600000000",
                    "last_updated": "1600000001",
                    "cover_photo": "https://example/0",
                    "photos": []
                }
            ]
        }"#;
        let parsed: AlbumsFile = serde_json::from_str(json).expect("parse");
        assert_eq!(parsed.albums.len(), 2);
        assert_eq!(parsed.albums[0].title, "Trip");
        assert_eq!(parsed.albums[0].photos, vec!["a", "b"]);
        assert_eq!(parsed.albums[1].photos.len(), 0);
    }

    #[test]
    fn parse_photo_fixture() {
        let json = r#"{
            "id": "54678248792",
            "name": "Colorado July 2025",
            "description": "With Heather and Scott",
            "count_views": "2",
            "date_taken": "2025-07-07 15:45:57",
            "rotation": 0,
            "photopage": "https://example/p",
            "original": "https://example/o.jpg",
            "license": "All Rights Reserved",
            "geo": [], "groups": [], "albums": [], "tags": [],
            "people": [], "notes": [], "comments": []
        }"#;
        let p: Photo = serde_json::from_str(json).expect("parse");
        assert_eq!(p.id, "54678248792");
        assert_eq!(p.name, "Colorado July 2025");
        assert_eq!(p.date_taken, "2025-07-07 15:45:57");
    }

    #[test]
    fn photo_with_missing_fields() {
        let json = r#"{ "id": "x" }"#;
        let p: Photo = serde_json::from_str(json).expect("parse");
        assert_eq!(p.id, "x");
        assert_eq!(p.name, "");
        assert_eq!(p.rotation, 0);
    }
}
