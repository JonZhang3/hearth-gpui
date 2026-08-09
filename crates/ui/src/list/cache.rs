use std::rc::Rc;

use gpui::{App, Pixels, Size};

use crate::IndexPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowEntry {
    Entry(IndexPath),
    SectionHeader(usize),
    SectionFooter(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct MeasuredEntrySize {
    pub(crate) item_size: Size<Pixels>,
    pub(crate) section_header_size: Size<Pixels>,
    pub(crate) section_footer_size: Size<Pixels>,
}

impl RowEntry {
    #[inline]
    #[allow(unused)]
    pub(crate) fn is_section_header(&self) -> bool {
        matches!(self, RowEntry::SectionHeader(_))
    }

    #[allow(unused)]
    pub(crate) fn index(&self) -> IndexPath {
        match self {
            RowEntry::Entry(index_path) => *index_path,
            RowEntry::SectionHeader(ix) => IndexPath::default().section(*ix),
            RowEntry::SectionFooter(ix) => IndexPath::default().section(*ix),
        }
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn is_section_footer(&self) -> bool {
        matches!(self, RowEntry::SectionFooter(_))
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn is_entry(&self) -> bool {
        matches!(self, RowEntry::Entry(_))
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn section_ix(&self) -> Option<usize> {
        match self {
            RowEntry::SectionHeader(ix) | RowEntry::SectionFooter(ix) => Some(*ix),
            _ => None,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct RowsCache {
    /// Only have section's that have rows.
    pub(crate) entities: Rc<Vec<RowEntry>>,
    pub(crate) items_count: usize,
    /// The sections, the item is number of rows in each section.
    pub(crate) sections: Rc<Vec<usize>>,
    /// Number of item rows before each section.
    section_item_offsets: Rc<Vec<usize>>,
    /// Flattened section-header position for each non-empty section.
    section_entity_offsets: Rc<Vec<Option<usize>>>,
    pub(crate) entries_sizes: Rc<Vec<Size<Pixels>>>,
    measured_size: MeasuredEntrySize,
}

impl RowsCache {
    pub(crate) fn get(&self, flatten_ix: usize) -> Option<RowEntry> {
        self.entities.get(flatten_ix).cloned()
    }

    /// Returns the number of flattened rows (Includes header, item, footer).
    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    /// Return the number of items in the cache.
    pub(crate) fn items_count(&self) -> usize {
        self.items_count
    }

    /// Returns the index of the  Entry with given path in the flattened rows.
    pub(crate) fn position_of(&self, path: &IndexPath) -> Option<usize> {
        let items_count = *self.sections.get(path.section)?;
        if path.row >= items_count {
            return None;
        }

        let section_header = self
            .section_entity_offsets
            .get(path.section)
            .copied()
            .flatten()?;
        Some(section_header + 1 + path.row)
    }

    /// Returns the flattened position of the first item row, skipping section
    /// headers and footers.
    pub(crate) fn first_entry_position(&self) -> Option<usize> {
        self.section_entity_offsets
            .iter()
            .flatten()
            .next()
            .map(|header| header + 1)
    }

    #[cfg(test)]
    pub(crate) fn first_entry(&self) -> Option<IndexPath> {
        self.entities.iter().find_map(|entry| match entry {
            RowEntry::Entry(ix) => Some(*ix),
            _ => None,
        })
    }

    #[cfg(test)]
    pub(crate) fn last_entry(&self) -> Option<IndexPath> {
        self.entities.iter().rev().find_map(|entry| match entry {
            RowEntry::Entry(ix) => Some(*ix),
            _ => None,
        })
    }

    /// Return the one-based item ordinal across all sections.
    pub(crate) fn item_ordinal(&self, path: IndexPath) -> Option<usize> {
        let items_count = *self.sections.get(path.section)?;
        if path.row >= items_count {
            return None;
        }

        Some(*self.section_item_offsets.get(path.section)? + path.row + 1)
    }

    /// Returns the previous item and wraps at the beginning.
    #[cfg(test)]
    pub(crate) fn prev(&self, path: Option<IndexPath>) -> IndexPath {
        let Some(path) = path else {
            return self.last_entry().unwrap_or_default();
        };
        let Some(pos) = self.position_of(&path) else {
            return self.last_entry().unwrap_or_default();
        };

        self.entities
            .iter()
            .take(pos)
            .rev()
            .find(|entry| entry.is_entry())
            .map(RowEntry::index)
            .unwrap_or_else(|| self.last_entry().unwrap_or_default())
    }

    /// Returns the next item and wraps at the end.
    #[cfg(test)]
    pub(crate) fn next(&self, path: Option<IndexPath>) -> IndexPath {
        let Some(path) = path else {
            return self.first_entry().unwrap_or_default();
        };
        let Some(pos) = self.position_of(&path) else {
            return self.first_entry().unwrap_or_default();
        };

        self.entities
            .iter()
            .skip(pos + 1)
            .find(|entry| entry.is_entry())
            .map(RowEntry::index)
            .unwrap_or_else(|| self.first_entry().unwrap_or_default())
    }

    pub(crate) fn prepare_if_needed<F>(
        &mut self,
        sections_count: usize,
        measured_size: MeasuredEntrySize,
        cx: &App,
        rows_count_f: F,
    ) where
        F: Fn(usize, &App) -> usize,
    {
        let mut new_sections = vec![];
        for section_ix in 0..sections_count {
            new_sections.push(rows_count_f(section_ix, cx));
        }

        let need_update = new_sections != *self.sections || self.measured_size != measured_size;

        if !need_update {
            return;
        }

        let mut entries_sizes = vec![];
        let mut section_item_offsets = Vec::with_capacity(new_sections.len());
        let mut section_entity_offsets = Vec::with_capacity(new_sections.len());
        let mut total_items_count = 0;
        let mut total_entities_count = 0;
        for items_count in new_sections.iter().copied() {
            section_item_offsets.push(total_items_count);
            section_entity_offsets.push((items_count > 0).then_some(total_entities_count));
            total_items_count += items_count;
            if items_count > 0 {
                total_entities_count += items_count + 2;
            }
        }

        self.measured_size = measured_size;
        self.sections = Rc::new(new_sections);
        self.section_item_offsets = Rc::new(section_item_offsets);
        self.section_entity_offsets = Rc::new(section_entity_offsets);
        self.entities = Rc::new(
            self.sections
                .iter()
                .enumerate()
                .flat_map(|(section, items_count)| {
                    let mut children = vec![];
                    if *items_count == 0 {
                        return children;
                    }

                    children.push(RowEntry::SectionHeader(section));
                    entries_sizes.push(measured_size.section_header_size);
                    for row in 0..*items_count {
                        children.push(RowEntry::Entry(IndexPath {
                            section,
                            row,
                            ..Default::default()
                        }));
                        entries_sizes.push(measured_size.item_size);
                    }
                    children.push(RowEntry::SectionFooter(section));
                    entries_sizes.push(measured_size.section_footer_size);
                    children
                })
                .collect(),
        );
        self.entries_sizes = Rc::new(entries_sizes);
        self.items_count = total_items_count;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        IndexPath,
        list::cache::{RowEntry, RowsCache},
    };

    fn build_entities(sections: &[usize]) -> Vec<RowEntry> {
        sections
            .iter()
            .enumerate()
            .flat_map(|(section, items_count)| {
                let mut children = vec![];
                if *items_count == 0 {
                    return children;
                }

                children.push(RowEntry::SectionHeader(section));
                for row in 0..*items_count {
                    children.push(RowEntry::Entry(IndexPath {
                        section,
                        row,
                        ..Default::default()
                    }));
                }
                children.push(RowEntry::SectionFooter(section));
                children
            })
            .collect()
    }

    fn build_cache(sections: &[usize]) -> RowsCache {
        let mut item_offset = 0;
        let mut entity_offset = 0;
        let mut section_item_offsets = Vec::with_capacity(sections.len());
        let mut section_entity_offsets = Vec::with_capacity(sections.len());

        for items_count in sections.iter().copied() {
            section_item_offsets.push(item_offset);
            section_entity_offsets.push((items_count > 0).then_some(entity_offset));
            item_offset += items_count;
            if items_count > 0 {
                entity_offset += items_count + 2;
            }
        }

        RowsCache {
            entities: build_entities(sections).into(),
            items_count: item_offset,
            sections: sections.to_vec().into(),
            section_item_offsets: section_item_offsets.into(),
            section_entity_offsets: section_entity_offsets.into(),
            ..Default::default()
        }
    }

    #[test]
    fn first_entry_position_skips_section_headers() {
        let row_cache = build_cache(&[2]);

        assert_eq!(row_cache.first_entry_position(), Some(1));
    }

    #[test]
    fn offsets_skip_empty_leading_sections() {
        let row_cache = build_cache(&[0, 0, 2]);

        assert_eq!(row_cache.next(None), IndexPath::new(0).section(2));
        assert_eq!(
            row_cache.position_of(&IndexPath::new(0).section(2)),
            Some(1),
            "the first real item must follow its section header"
        );
        assert_eq!(
            row_cache.item_ordinal(IndexPath::new(1).section(2)),
            Some(2)
        );
    }

    #[test]
    fn item_ordinal_is_global_across_sections() {
        let row_cache = build_cache(&[2, 0, 3]);

        assert_eq!(
            row_cache.item_ordinal(IndexPath::new(1).section(2)),
            Some(4)
        );
    }

    #[test]
    fn test_prev_next() {
        let row_cache = build_cache(&[2, 4, 3]);

        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(0))),
            IndexPath::new(1).section(0)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(1).section(0))),
            IndexPath::new(0).section(1)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(1))),
            IndexPath::new(1).section(1)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(3).section(1))),
            IndexPath::new(0).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(2))),
            IndexPath::new(1).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(1).section(2))),
            IndexPath::new(2).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(2).section(2))),
            IndexPath::new(0).section(0)
        );

        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(0))),
            IndexPath::new(2).section(2)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(1).section(0))),
            IndexPath::new(0).section(0)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(1))),
            IndexPath::new(1).section(0)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(1).section(1))),
            IndexPath::new(0).section(1)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(3).section(1))),
            IndexPath::new(2).section(1)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(2))),
            IndexPath::new(3).section(1)
        );
    }

    #[test]
    fn test_prev_next_with_empty_sections() {
        let row_cache = build_cache(&[2, 0, 3, 0, 1]);

        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(0))),
            IndexPath::new(1).section(0)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(1).section(0))),
            IndexPath::new(0).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(2))),
            IndexPath::new(1).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(2).section(2))),
            IndexPath::new(0).section(4)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(4))),
            IndexPath::new(0).section(0)
        );

        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(0))),
            IndexPath::new(0).section(4)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(2))),
            IndexPath::new(1).section(0)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(4))),
            IndexPath::new(2).section(2)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(1).section(2))),
            IndexPath::new(0).section(2)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(2).section(2))),
            IndexPath::new(1).section(2)
        );
    }
}
