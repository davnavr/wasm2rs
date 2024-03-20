#[derive(Clone, Copy, Debug)]
pub struct ExportEntry {
    pub name: u32,
    pub index: u32,
}

#[derive(Default, Debug)]
pub struct Exports<'a> {
    pub names: Vec<&'a str>,
    pub functions: Vec<ExportEntry>,
}

//fn write_exports
