use crate::{
    error::{Error, Result},
    utils,
    vault::link::{create_link, for_each_internal_link},
    Edge, Note, NoteId, Vault,
    vault::note::FromHash,
};
use anyhow::Context;
use ignore::{overrides::OverrideBuilder, types::TypesBuilder, WalkBuilder};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Builder struct for making a Vault instance.
#[derive(Default)]
pub struct VaultBuilder {
    source: PathBuf,
    ignores: Vec<String>,
}

impl VaultBuilder {
    /// Set the source for the directory walker.
    ///
    /// The source has to be a directory, but this function does not check that. Rather, the
    /// [`build`](VaultBuilder::build) function will do a check and errors if it fails.
    #[must_use]
    pub fn source(self, source: impl AsRef<Path>) -> Self {
        Self {
            source: source.as_ref().to_owned(),
            ..self
        }
    }

    /// Set the ignore patterns for the directory walker.
    ///
    /// The patterns follows the [gitignore format](https://git-scm.com/docs/gitignore).
    #[must_use]
    pub fn ignores(self, ignores: Vec<String>) -> Self {
        Self { ignores, ..self }
    }

    /// Build a [`Vault`] from the options supplied to the builder.
    pub fn build(&self) -> Result<Vault> {
        if !self.source.is_dir() {
            return Err(Error::VaultSourceNotDir);
        }

        let walker = {
            let mut overrides = OverrideBuilder::new(&self.source);
            for ignore in self.ignores.iter() {
                if let Some(s) = ignore.strip_prefix('!') {
                    overrides
                        .add(s)
                        .with_context(|| format!("Invalid ignore pattern: {}", ignore))?;
                } else {
                    overrides
                        .add(&format!("!{}", ignore))
                        .with_context(|| format!("Invalid ignore pattern: {}", ignore))?;
                }
            }

            WalkBuilder::new(&self.source)
                .hidden(true)
                .overrides(
                    overrides
                        .build()
                        .context("Building walker overrides failed.")?,
                )
                .types(
                    TypesBuilder::new()
                        .add_defaults()
                        .select("markdown")
                        .build()
                        .expect("Building default types should never fail."),
                )
                .build()
        };

        let mut notes = walker
            .filter_map(|e| e.ok())
            .filter(|e| !e.path().is_dir())
            .map(|e| e.into_path())
            .map(|path| {
                let path_from_root = utils::fs::diff_paths(&path, &self.source).unwrap();
                let id = NoteId::from_hash(&path_from_root);

                let mut note = Note {
                    title: path.file_stem().unwrap().to_string_lossy().to_string(),
                    path: Some(path_from_root),
                    tags: vec![],
                    date: None,
                    content: utils::fs::read_file(path)?,
                    adjacencies: HashMap::new(),
                };

                note.process_front_matter()?;

                Ok((id, note))
            })
            .collect::<Result<HashMap<NoteId, Note>>>()?;

        let adjacencies: HashMap<NoteId, Edge> = notes
            .keys()
            .clone()
            .map(|id| (*id, Edge::NotConnected))
            .collect();

        let id_lookup: HashMap<String, NoteId> = notes
            .clone()
            .into_iter()
            .flat_map(|(id, note)| {
                let mut lookup_entries = vec![(note.title, id)];
                if let Some(path) = note.path {
                    // This allows linking by filename
                    lookup_entries.push((path.to_string_lossy().to_string(), id));
                }
                lookup_entries
            })
            .collect();

        let path_lookup: HashMap<NoteId, PathBuf> = notes
            .clone()
            .into_iter()
            .map(|(id, note)| (id, note.path.unwrap()))
            .collect();

        notes.iter_mut().try_for_each(|(_, note)| {
            note.adjacencies = adjacencies.clone();
            for_each_internal_link(&note.content.clone(), |link_string| {
                match create_link(link_string, &path_lookup, &id_lookup) {
                    Ok(link) => {
                        note.adjacencies
                            .entry(link.dest_id)
                            .and_modify(|adj| *adj = Edge::Connected);
                        note.content = note.content.replacen(
                            &format!("[[{}]]", link_string),
                            &link.cmark(note.path.as_ref().unwrap().parent().unwrap()),
                            1,
                        );
                    }
                    Err(Error::InvalidInternalLinkDestination(_)) => {}
                    Err(e) => return Err(e),
                }

                Ok(())
            })
        })?;

        Ok(Vault { notes, id_lookup })
    }
}
