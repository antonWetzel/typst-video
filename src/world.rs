use std::sync::OnceLock;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use comemo::Prehashed;
use fontdb::Database;

use chrono::{DateTime, Datelike, FixedOffset, Local, Utc};
use typst::diag::{PackageError, PackageResult};
use typst::syntax::package::PackageSpec;
use typst::text::{Font, FontBook, FontInfo};
use typst::{
    diag::{FileError, FileResult, SourceResult},
    eval::Tracer,
    foundations::Dict,
    model::Document,
    syntax::{FileId, Source, VirtualPath},
    Library, World,
};

#[derive(Debug)]
pub struct VideoWorld {
    library: Prehashed<Library>,
    now: DateTime<Utc>,
    main: FileId,
    root: PathBuf,
    font_manager: FontManager,
    shadow_files: HashMap<FileId, Source>,
}

impl VideoWorld {
    pub fn new(main: PathBuf, root: Option<PathBuf>) -> Self {
        let root = root.unwrap_or_else(|| main.parent().unwrap().to_path_buf());
        let main = VirtualPath::new(main.strip_prefix(&root).unwrap());
        let inputs = Dict::new();

        Self {
            library: Prehashed::new(Library::builder().with_inputs(inputs).build()),
            now: chrono::Utc::now(),
            font_manager: FontManager::new(),
            main: FileId::new(None, main),
            root,
            shadow_files: HashMap::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, file_id: FileId) -> typst::diag::FileResult<PathBuf> {
        let path = if let Some(spec) = file_id.package() {
            prepare_package(spec)?.join(file_id.vpath().as_rootless_path())
        } else {
            self.root.join(file_id.vpath().as_rootless_path())
        };

        Ok(path)
    }

    pub fn compile(&self) -> SourceResult<Document> {
        let mut tracer = Tracer::new();
        typst::compile(self, &mut tracer)
    }
}

impl World for VideoWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn today(&self, offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        let with_offset = match offset {
            None => self.now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                self.now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        typst::foundations::Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }

    fn book(&self) -> &Prehashed<typst::text::FontBook> {
        self.font_manager.book()
    }

    fn main(&self) -> typst::syntax::Source {
        self.source(self.main).unwrap()
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<typst::syntax::Source> {
        if let Some(source) = self.shadow_files.get(&id) {
            return Ok(source.clone());
        }

        let path = self.path(id)?;

        let Ok(text) = std::fs::read_to_string(&path) else {
            return Err(FileError::NotFound(path));
        };
        Ok(Source::new(id, text))
    }

    fn file(&self, id: FileId) -> FileResult<typst::foundations::Bytes> {
        let path = self.path(id)?;

        let Ok(bytes) = std::fs::read(&path) else {
            return Err(FileError::NotFound(path));
        };
        Ok(bytes.into())
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.font_manager.get(index)
    }
}

/// Make a package available in the on-disk cache.
/// Just hope the preview packages are already donwloaded.
pub fn prepare_package(spec: &PackageSpec) -> PackageResult<PathBuf> {
    let subdir = format!(
        "typst/packages/{}/{}/{}",
        spec.namespace, spec.name, spec.version
    );

    if let Some(data_dir) = dirs::data_dir() {
        let dir = data_dir.join(&subdir);
        if dir.exists() {
            return Ok(dir);
        }
    }

    if let Some(cache_dir) = dirs::cache_dir() {
        let dir = cache_dir.join(&subdir);
        if dir.exists() {
            return Ok(dir);
        }
    }

    Err(PackageError::NotFound(spec.clone()))
}

#[derive(Debug)]
pub struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    pub fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = std::fs::read(&self.path).ok()?.into();
                Font::new(data, self.index)
            })
            .clone()
    }
}

#[derive(Debug)]
pub struct FontManager {
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
}

impl FontManager {
    pub fn new() -> Self {
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        let mut db = Database::new();
        db.load_system_fonts();

        for face in db.faces() {
            let path = match &face.source {
                fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => path,
                fontdb::Source::Binary(_) => continue,
            };

            let info = db
                .with_face_data(face.id, FontInfo::new)
                .expect("database must contain this font");

            if let Some(info) = info {
                book.push(info);
                fonts.push(FontSlot {
                    path: path.clone(),
                    index: face.index,
                    font: OnceLock::new(),
                });
            }
        }

        for data in typst_assets::fonts() {
            let buffer = typst::foundations::Bytes::from_static(data);
            for (i, font) in Font::iter(buffer).enumerate() {
                book.push(font.info().clone());
                fonts.push(FontSlot {
                    path: PathBuf::new(),
                    index: i as u32,
                    font: OnceLock::from(Some(font)),
                });
            }
        }

        Self {
            book: Prehashed::new(book),
            fonts,
        }
    }

    pub fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    pub fn get(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }
}
