use crate::yaml::{
    lookup_value,
    tostr,
    YamlMap,
    YamlValue,
    to_iterable,
    insert_value,
    YamlFileError,
};
use crate::parsers::parse_template_string;
use crate::io::{ReadsFiles, FileError};
use crate::utils::{map_m};
use crate::pipes::{
    Pipe, PipeMap, execute_pipes, PipeInputSource
};

pub struct TemplateContext {
    pub snippet_folder: String,
    pub output_folder: String,
}

pub fn default_template_context() -> TemplateContext {
    TemplateContext {
        snippet_folder: "resources/snippets/".to_owned(),
        output_folder:"build/".to_owned(),
    } 
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TemplateElement {
    PlainText(String),
    Replace { value: TemplateValue, pipe: Vec<Pipe> },
    File { snippet: bool, filename: String, pipe: Vec<Pipe> },
    FileAt { snippet: bool, value: TemplateValue, value_pipe: Vec<Pipe>, contents_pipe: Vec<Pipe> },
    IfExists {
        value: TemplateValue,
        when_true: Vec<TemplateElement>,
        when_false: Vec<TemplateElement>
    },
    For {
        name: String,
        main: Vec<TemplateElement>,
        groupings: Vec<ForGrouping>,
        separator: Vec<TemplateElement>,
        sort_and_filter: ForSortAndFilter,
    },
    LookupCatcher(Vec<Vec<TemplateElement>>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForGrouping {
    pub values: Vec<TemplateValue>,
    pub filenames: Vec<String>,
    pub files_at: Vec<TemplateValue>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForSortAndFilter {
    pub sort_key: Option<TemplateValue>,
    pub is_sort_ascending: bool,
    pub filter_includes: Option<TemplateValue>,
    pub filter_excludes: Option<TemplateValue>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TemplateValue {
    pub base: String,
    pub accesses: Vec<TemplateValueAccess>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TemplateValueAccess {
    Field(String),
    Index(usize)
}

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateError {
    FileError(FileError),
    FileErrorDerivedFrom(FileError, TemplateValue),
    YamlFileError(YamlFileError),
    OnForLoopIteration(Box<TemplateError>, String),
    InIfExistsLoop(Box<TemplateError>, TemplateValue, bool),
    KeyNotPresent(String),
    KeyNotString(String),
    ParseError(String),
    SerialisationError(String),
    IndexOOB(String, usize),
    FieldNotPresent(String, String),
    IndexOnUnindexable(String, usize),
    FieldOnUnfieldable(String, String),
    ForOnUnindexable(String),
    PipeMissing(String),
    PipeExecutionError(String, String, usize, String),
    WithinTemplatePipe(Box<TemplateError>, usize, String),
    WithinTemplateNamedPipe(Box<TemplateError>, String, usize, String),
    UnknownError,
}

impl std::fmt::Display for TemplateValue {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(ff, "{}{}", self.base, Vec::from_iter(self.accesses.iter().map(|ii| ii.to_string())).join(""))
    }
}

impl std::fmt::Display for TemplateValueAccess {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateValueAccess::Field(strr) => write!(ff, ".{}", strr),
            TemplateValueAccess::Index(strr) => write!(ff, ".{}", strr),
        }
    }
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::UnknownError => write!(ff, "An unknown error occured."),
            TemplateError::FileError(err) => err.fmt(ff),
            TemplateError::FileErrorDerivedFrom(err, value) => write!(ff, "{} (derived from {})", err, value),
            TemplateError::YamlFileError(err) => err.fmt(ff),
            TemplateError::OnForLoopIteration(err, value) => write!(ff, "{}\nwithin for loop entry {}", err, value),
            TemplateError::InIfExistsLoop(err, value, truthiness) => write!(ff, "{}\nwithin the {} branch of checking if {} exists", err, truthiness, value),
            TemplateError::KeyNotPresent(strr) => write!(ff, "The key {} was not present in the parameters.", strr),
            TemplateError::KeyNotString(strr) => write!(ff, "The key {} in the parameters was not a string.", strr),
            TemplateError::ParseError(strr) => write!(ff, "Parsing the templating text failed: {}", strr),
            TemplateError::SerialisationError(strr) => write!(ff, "Failed to serialise a value: {}", strr),
            TemplateError::IndexOOB(strr, idx) => write!(ff, "Index {} of {} is out of bounds", idx, strr),
            TemplateError::IndexOnUnindexable(strr, idx) => write!(ff, "{} isn't indexable, so can't be indexed at {}", strr, idx),
            TemplateError::FieldNotPresent(strr, idx) => write!(ff, "{} has no property named {}", strr, idx),
            TemplateError::FieldOnUnfieldable(strr, idx) => write!(ff, "{} has no properties, so field {} can't be accessed", strr, idx),
            TemplateError::ForOnUnindexable(strr) => write!(ff, "Can't do a for loop on {} as it's not indexable", strr),
            TemplateError::PipeMissing(strr) => write!(ff, "Can't use pipe {} as it doesn't exist", strr),
            TemplateError::PipeExecutionError(error, pipename, pipeindex, path) => 
                write!(ff, "Error running pipe {} as pipe {} on {}: {}", pipename, pipeindex, path, error),
            TemplateError::WithinTemplatePipe(error, pipeindex, path) => 
                write!(ff, "{}\nwhen running the default templating pipe as pipe {} on {}", error, pipeindex, path),
            TemplateError::WithinTemplateNamedPipe(error, pipename, pipeindex, path) => 
                write!(ff, "{}\nwhen running templating pipe {} as pipe {} on {}", error, pipename, pipeindex, path),
        }
    }
}

impl TemplateElement {
    fn render<'a>(&'a self, params: &'a YamlMap, pipes: &'a PipeMap, io: &mut impl ReadsFiles, context: &TemplateContext) -> Result<String, TemplateError> {
        match self {
            TemplateElement::PlainText(text) => Ok(text.clone()),
            TemplateElement::Replace{value, pipe} => {
                let lookup = lookup_value(value, params)?;
                let piped = execute_pipes(lookup, &pipe, params, PipeInputSource::Value(&value), pipes, io, context)?;
                tostr(&piped)
            },
            TemplateElement::File{snippet, filename, pipe} => {
                let real_filename = format!("{}{}", if *snippet {&context.snippet_folder} else {""}, filename);
                match io.read(&real_filename) {
                    Ok(strr) => {
                        let piped = execute_pipes(
                            &YamlValue::String(strr.to_owned()), pipe, params,
                            PipeInputSource::File(&real_filename), pipes, io, context
                        )?;
                        tostr(&piped)
                    },
                    Err(ee) => Err(TemplateError::FileError(ee))
                }
            },
            TemplateElement::FileAt{snippet, value, value_pipe, contents_pipe} => {
                let lookup = lookup_value(value, params)?;
                let piped_filename = execute_pipes(
                    lookup, value_pipe, params, PipeInputSource::Value(&value), pipes, io, context
                )?;
                let filename = tostr(&piped_filename)?;
                let real_filename = format!("{}{}", if *snippet {&context.snippet_folder} else {""}, filename);
                match io.read(&real_filename) {
                    Ok(strr) => {
                        let piped = execute_pipes(
                            &YamlValue::String(strr.to_owned()), contents_pipe, params,
                            PipeInputSource::FileFrom(&real_filename, &value), pipes, io, context
                        )?;
                        tostr(&piped)
                    },
                    Err(ee) => Err(TemplateError::FileErrorDerivedFrom(ee, value.clone()))
                }
            }
            TemplateElement::IfExists{value, when_true, when_false} => {
                let lookup = lookup_value(value, params);
                match lookup {
                    Ok(..) => render_elements(when_true, params, pipes, io, context)
                        .map_err(|ee| TemplateError::InIfExistsLoop(Box::new(ee), value.clone(), true)),
                    Err(ee) => match ee {
                        TemplateError::KeyNotPresent(..) |
                        TemplateError::FieldNotPresent(..) |
                        TemplateError::IndexOOB(..) => render_elements(when_false, params, pipes, io, context)
                            .map_err(|ee| TemplateError::InIfExistsLoop(Box::new(ee), value.clone(), false)),
                        _ => Err(ee)
                    }
                }
            }
            TemplateElement::LookupCatcher(values) => {
                let mut rendered: Result<String, TemplateError> = Err(TemplateError::UnknownError);
                let mut stop = false;
                let mut iter = values.iter();
                while let Some(value) = iter.next() {
                    if !stop && rendered.is_err() {
                        match render_elements(value, params, pipes, io, context) {
                            ee @ Err(TemplateError::KeyNotPresent(..)) |
                            ee @ Err(TemplateError::FieldNotPresent(..)) |
                            ee @ Err(TemplateError::IndexOOB(..)) => {
                                rendered = ee;
                            },
                            ee @ Err(_) => {
                                rendered = ee;
                                stop = true;
                            }
                            aa @ Ok(_) => {
                                rendered = aa;
                            }
                        }
                    }
                }
                rendered
            }
            TemplateElement::For{name, groupings, main, separator, ..} => {
                let over = for_make_iterable(params, groupings, io)?;
                let mapped: Vec<String> = map_m(over, |ii| {
                    let mut new_params = params.clone();
                    insert_value(&mut new_params, &name, ii.0.clone());
                    render_elements(main, &new_params, pipes, io, context)
                        .map_err(|ee| TemplateError::OnForLoopIteration(Box::new(ee), ii.1.to_string()))
                })?;
                let sep = render_elements(separator, params, pipes, io, context)?;
                Ok(mapped.join(&sep))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ForIterationType<'a>{
    Values(&'a TemplateValue, usize),
    Filenames(&'a str, usize),
    FileAt(&'a TemplateValue, String, usize),
}

impl<'a> std::fmt::Display for ForIterationType<'a> {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForIterationType::FileAt(value, filename, size) => write!(ff, "{} of file {} derived from {}", size, filename, value),
            ForIterationType::Filenames(filename, size) => write!(ff, "{} of file {}", size, filename),
            ForIterationType::Values(value, size) => write!(ff, "{} of {}", size, value),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ForIteration<'a>(YamlValue, ForIterationType<'a>);

fn for_make_iterable<'a>(
    params: & YamlMap,
    groupings: &'a Vec<ForGrouping>,
    io: &mut impl ReadsFiles
) -> Result<Vec<ForIteration<'a>>, TemplateError> {
    let mut entries: Vec<ForIteration> = Vec::new();
    for gg in groupings {
        for value in gg.values.iter() {
            let lookup = lookup_value(&value, params)?;
            let as_vec = to_iterable(lookup)?;
            for (ind, finalval) in as_vec.into_iter().enumerate() {
                entries.push(ForIteration(finalval, ForIterationType::Values(&value, ind)));
            }
        }
        for filename in gg.filenames.iter() {
            let lookup = io.read_yaml(&filename)
                .map_err(|xx| TemplateError::YamlFileError(xx))?;
            let as_vec = to_iterable(lookup)?;
            for (ind, finalval) in as_vec.into_iter().enumerate() {
                entries.push(ForIteration(finalval, ForIterationType::Filenames(&filename, ind)));
            }
        }
        for fileat in gg.files_at.iter() {
            let lookup = lookup_value(&fileat, params)?;
            let filename = tostr(lookup)?;
            let file = io.read_yaml(&filename)
                .map_err(|xx| TemplateError::YamlFileError(xx))?;
            let as_vec = to_iterable(file)?;
            for (ind, finalval) in as_vec.into_iter().enumerate() {
                entries.push(ForIteration(finalval, ForIterationType::FileAt(&fileat, filename.to_owned(), ind)));
            }
        }
    }
    Ok(entries)
}

pub fn render_elements<'a>(
    elements: &'a Vec<TemplateElement>,
    params: &'a YamlMap,
    pipes: &'a PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext
) -> Result<String, TemplateError> {
    elements.iter().try_fold("".to_owned(), |acc, ii| {
        match ii.render(params, pipes, io, context) {
            err @ Err(..) => err,
            Ok(result) => {
                let mut string = acc.to_owned();
                string.push_str(&result);
                Ok(string)
            }
        }
    })
}

pub fn render<'a>(
    input: &'a str,
    params: &'a YamlMap,
    pipes: &'a PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext
) -> Result<String, TemplateError> {
    match parse_template_string(input) {
        Err(ee) => Err(TemplateError::ParseError(ee.to_string())),
        Ok(elements) => render_elements(&elements, params, pipes, io, context)
    }
}
