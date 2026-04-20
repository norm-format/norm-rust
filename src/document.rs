#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Document {
    pub(crate) root_array: bool,
    pub(crate) sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section {
    pub(crate) name: String,
    pub(crate) array: bool,
    pub(crate) header_line: usize,
    pub(crate) header: Vec<String>,
    pub(crate) rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) line: usize,
    pub(crate) cells: Vec<crate::lexer::Cell>,
}

impl Document {
    pub(crate) fn root_section(&self) -> Option<&Section> {
        self.sections.first()
    }

    pub(crate) fn find_section(&self, name: &str) -> Option<(usize, &Section)> {
        self.sections
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == name)
    }
}
