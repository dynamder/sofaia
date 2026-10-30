use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Folder,
    File(DataType),
    SymLink,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Text,
    Image,
    Audio,
    Video,
    Archive,
    Other,
}

impl ItemType {
    pub fn is_folder(self) -> bool {
        matches!(self, ItemType::Folder)
    }

    pub fn suffix(self) -> &'static str {
        match self {
            ItemType::Folder => "/",
            ItemType::SymLink => "@",
            _ => "",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ItemType::Folder => "Folder",
            ItemType::File(DataType::Text) => "Text",
            ItemType::File(DataType::Image) => "Image",
            ItemType::File(DataType::Audio) => "Audio",
            ItemType::File(DataType::Video) => "Video",
            ItemType::File(DataType::Archive) => "Archive",
            ItemType::File(DataType::Other) => "File",
            ItemType::SymLink => "Symlink",
            ItemType::Unknown => "Unknown",
        }
    }
}

impl DataType {
    pub fn detect(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        match extension.as_str() {
            "txt" | "md" | "rs" | "toml" | "json" | "yaml" | "yml" | "sh" | "py" | "js" | "ts"
            | "c" | "h" | "cpp" | "hpp" | "go" | "html" | "css" | "kdl" => DataType::Text,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "avif" | "tiff" => {
                DataType::Image
            }
            "mp3" | "flac" | "ogg" | "wav" | "m4a" | "opus" => DataType::Audio,
            "mp4" | "mkv" | "webm" | "mov" | "avi" => DataType::Video,
            "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" | "bz2" => DataType::Archive,
            _ => DataType::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    path: PathBuf,
    item_type: ItemType,
}

impl Entry {
    pub fn new(path: PathBuf, item_type: ItemType) -> Self {
        Self { path, item_type }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn item_type(&self) -> ItemType {
        self.item_type
    }

    pub fn name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    pub fn label(&self) -> String {
        format!("{}{}", self.name(), self.item_type.suffix())
    }
}

pub async fn read_dir(path: PathBuf) -> io::Result<Vec<Entry>> {
    let mut dir = tokio::fs::read_dir(path).await?;
    let mut entries = Vec::new();

    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();

        let item_type = match entry.file_type().await {
            Ok(file_type) if file_type.is_dir() => ItemType::Folder,
            Ok(file_type) if file_type.is_file() => ItemType::File(DataType::detect(&path)),
            Ok(_) => ItemType::SymLink,
            Err(_) => ItemType::Unknown,
        };

        entries.push(Entry::new(path, item_type));
    }

    entries.sort_by_cached_key(|entry| (!entry.item_type.is_folder(), entry.name().to_lowercase()));

    Ok(entries)
}
