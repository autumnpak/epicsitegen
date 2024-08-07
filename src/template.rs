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
use crate::pipes::{
    Pipe, PipeMap, execute_pipes, PipeInputSource, pipe_cache_check, CacheStatus
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
    Into {
        value: TemplateValue,
        ast: Vec<TemplateElement>,
    },
    LookupCatcher(Vec<Vec<TemplateElement>>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForGrouping {
    pub values: Vec<TemplateValueWithPipe>,
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
pub struct TemplateValueWithPipe {
    pub value: TemplateValue,
    pub pipes: Vec<Pipe>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TemplateValueAccess {
    Field(String),
    Index(usize),
    IndexAt(TemplateValue),
}

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateError {
    FileError(FileError),
    FileErrorDerivedFrom(FileError, TemplateValue),
    YamlFileError(YamlFileError),
    OnForLoopIteration(Box<TemplateError>, String),
    OnForLoopIterationSortKey(Box<TemplateError>, String),
    OnForLoopIterationIncludeKey(Box<TemplateError>, String),
    OnForLoopIterationExcludeKey(Box<TemplateError>, String),
    InIfExistsLoop(Box<TemplateError>, TemplateValue, bool),
    InIntoStatement(Box<TemplateError>, TemplateValue),
    IntoValueNotHash(TemplateValue),
    KeyNotPresent(String),
    KeyNotString(String),
    ParseError(String),
    SerialisationError(String),
    IndexOOB(String, usize),
    IndexWithNonIntegerValue(String, TemplateValue),
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
            TemplateValueAccess::Index(strr) => write!(ff, "[{}]", strr),
            TemplateValueAccess::IndexAt(strr) => write!(ff, "[{}]", strr),
        }
    }
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::UnknownError => write!(ff, "An unknown error occured."),
            TemplateError::FileError(err) => err.fmt(ff),
            TemplateError::FileErrorDerivedFrom(err, value) => write!(ff, "{} (derived from \"{}\")", err, value),
            TemplateError::YamlFileError(err) => err.fmt(ff),
            TemplateError::OnForLoopIteration(err, value) => write!(ff, "{}\nwithin for loop entry {}", err, value),
            TemplateError::OnForLoopIterationSortKey(err, value) => write!(ff, "{}\nwhen getting the sorting key for loop entry {}", err, value),
            TemplateError::OnForLoopIterationIncludeKey(err, value) => write!(ff, "{}\nwhen getting the include key for loop entry {}", err, value),
            TemplateError::OnForLoopIterationExcludeKey(err, value) => write!(ff, "{}\nwhen getting the exclude key for loop entry {}", err, value),
            TemplateError::InIfExistsLoop(err, value, truthiness) => write!(ff, "{}\nwithin the {} branch of checking if {} exists", err, truthiness, value),
            TemplateError::InIntoStatement(err, value) => write!(ff, "{}\nwhen params are derived from {}", err, value),
            TemplateError::IntoValueNotHash(value) => write!(ff, "Can't use {} with an into statement as it's not an object", value),
            TemplateError::KeyNotPresent(strr) => write!(ff, "The key \"{}\" was not present in the parameters.", strr),
            TemplateError::KeyNotString(strr) => write!(ff, "The key \"{}\" in the parameters was not a string.", strr),
            TemplateError::ParseError(strr) => write!(ff, "Parsing the templating text failed: {}", strr),
            TemplateError::SerialisationError(strr) => write!(ff, "Failed to serialise a value: {}", strr),
            TemplateError::IndexOOB(strr, idx) => write!(ff, "Index {} of {} is out of bounds", idx, strr),
            TemplateError::IndexWithNonIntegerValue(strr, idx) => write!(ff, "{} isn't an integer, so it can't index {}", idx, strr),
            TemplateError::IndexOnUnindexable(strr, idx) => write!(ff, "{} isn't indexable, so can't be indexed at {}", strr, idx),
            TemplateError::FieldNotPresent(strr, idx) => write!(ff, "{} has no property named {}", strr, idx),
            TemplateError::FieldOnUnfieldable(strr, idx) => write!(ff, "{} has no properties, so field {} can't be accessed", strr, idx),
            TemplateError::ForOnUnindexable(strr) => write!(ff, "Can't do a for loop on {} as it's not indexable", strr),
            TemplateError::PipeMissing(strr) => write!(ff, "Can't use pipe {} as it doesn't exist", strr),
            TemplateError::PipeExecutionError(error, pipename, pipeindex, path) => 
                write!(ff, "Error running pipe \"{}\" as pipe {} on {}: {}", pipename, pipeindex, path, error),
            TemplateError::WithinTemplatePipe(error, pipeindex, path) => 
                write!(ff, "{}\nwhen running the default templating pipe as pipe {} on {}", error, pipeindex, path),
            TemplateError::WithinTemplateNamedPipe(error, pipename, pipeindex, path) => 
                write!(ff, "{}\nwhen running templating pipe \"{}\" as pipe {} on {}", error, pipename, pipeindex, path),
        }
    }
}

fn get_file2<'a>(
    filename: &'a str,
    pipes: &'a Vec<Pipe>,
    params: &'a YamlMap, 
    pipemap: &'a PipeMap, 
    io: &mut impl ReadsFiles, 
    context: &TemplateContext
) -> Result<String, TemplateError> {
    match io.read(filename) {
        Ok(strr) => {
            let piped = execute_pipes(
                &YamlValue::String(strr.to_owned()), pipes, params,
                PipeInputSource::File(filename), pipemap, io, context
            )?;
            tostr(&piped)
        },
        Err(ee) => Err(TemplateError::FileError(ee))
    }
}

fn get_file<'a>(
    filename: &'a str,
    pipes: &'a Vec<Pipe>,
    params: &'a YamlMap, 
    pipemap: &'a PipeMap, 
    io: &mut impl ReadsFiles, 
    context: &TemplateContext
) -> Result<String, TemplateError> {
    match pipe_cache_check(filename, pipes, pipemap, io) {
        CacheStatus::UpToDate(strr) => {
            match io.read(&strr) {
                Ok(strr) => { Ok(strr.to_owned()) },
                Err(ee) => Err(TemplateError::FileError(ee))
            }
        },
        CacheStatus::NeedsUpdate(strr) => {
            let result = get_file2(filename, pipes, params, pipemap, io, context)?;
            io.write(&strr, &result).map_err(|ee| TemplateError::FileError(ee))?;
            Ok(result)
        },
        CacheStatus::Uncachable => {
            get_file2(filename, pipes, params, pipemap, io, context)
        }
    }
}

impl TemplateElement {
    fn render<'a>(&'a self, params: &'a YamlMap, pipes: &'a PipeMap, io: &mut impl ReadsFiles, context: &TemplateContext) -> Result<String, TemplateError> {
        match self {
            TemplateElement::PlainText(text) => Ok(text.clone()),
            TemplateElement::Replace{value, pipe} => {
                let lookup = lookup_value(value, params)?;
                let piped = execute_pipes(&lookup, &pipe, params, PipeInputSource::Value(&value), pipes, io, context)?;
                tostr(&piped)
            },
            TemplateElement::File{snippet, filename, pipe} => {
                let real_filename = format!("{}{}", if *snippet {&context.snippet_folder} else {""}, filename);
                get_file(&real_filename, pipe, params, pipes, io, context)
            },
            TemplateElement::FileAt{snippet, value, value_pipe, contents_pipe} => {
                let lookup = lookup_value(value, params)?;
                let piped_filename = execute_pipes(
                    &lookup, value_pipe, params, PipeInputSource::Value(&value), pipes, io, context
                )?;
                let filename = tostr(&piped_filename)?;
                let real_filename = format!("{}{}", if *snippet {&context.snippet_folder} else {""}, filename);
                get_file(&real_filename, contents_pipe, params, pipes, io, context)
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
            TemplateElement::Into{value, ast} => {
                let lookup = lookup_value(value, params);
                match lookup {
                    Ok(YamlValue::Hash(hh)) => render_elements(ast, &hh, pipes, io, context)
                        .map_err(|ee| TemplateError::InIntoStatement(Box::new(ee), value.clone())),
                    Ok(_) => Err(TemplateError::IntoValueNotHash(value.clone())),
                    Err(ee) => Err(TemplateError::InIntoStatement(Box::new(ee), value.clone()))
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
            TemplateElement::For{name, groupings, main, separator, sort_and_filter} => {
                let over = for_make_iterable(params, groupings, pipes, io, context)?;
                let mut mapped: Vec<(String, String)> = Vec::new();
                for ii in over {
                    let mut new_params = params.clone();
                    insert_value(&mut new_params, &name, ii.0.clone());
                    let included = if let Some(ss) = &sort_and_filter.filter_includes {
                        let keylookup = lookup_value(&ss, &new_params);
                        match keylookup {
                            Ok(..) => true,
                            Err(TemplateError::KeyNotPresent(..)) |
                            Err(TemplateError::FieldNotPresent(..)) |
                            Err(TemplateError::IndexOOB(..)) => { false },
                            Err(ee) => Err(TemplateError::OnForLoopIterationIncludeKey(Box::new(ee), ii.1.to_string()))?,
                        }
                    } else {
                        true
                    };
                    let excluded = if let Some(ss) = &sort_and_filter.filter_excludes {
                        let keylookup = lookup_value(&ss, &new_params);
                        match keylookup {
                            Ok(..) => false,
                            Err(TemplateError::KeyNotPresent(..)) |
                            Err(TemplateError::FieldNotPresent(..)) |
                            Err(TemplateError::IndexOOB(..)) => { true },
                            Err(ee) => Err(TemplateError::OnForLoopIterationExcludeKey(Box::new(ee), ii.1.to_string()))?,
                        }
                    } else {
                        true
                    };
                    if included && excluded {
                        let key = match &sort_and_filter.sort_key {
                            None => String::new(),
                            Some(ss) => {
                                let keylookup = lookup_value(&ss, &new_params)
                                    .map_err(|ee| TemplateError::OnForLoopIterationSortKey(Box::new(ee), ii.1.to_string()))?;
                                tostr(&keylookup)
                                    .map_err(|ee| TemplateError::OnForLoopIterationSortKey(Box::new(ee), ii.1.to_string()))?
                            }
                        };
                        let value = render_elements(main, &new_params, pipes, io, context)
                            .map_err(|ee| TemplateError::OnForLoopIteration(Box::new(ee), ii.1.to_string()))?;
                        mapped.push((key, value))
                    }
                };
                if sort_and_filter.sort_key.is_some() {
                    mapped.sort_by(|aa, bb| {
                        let oo = (&aa.0).cmp(&bb.0);
                        if sort_and_filter.is_sort_ascending { oo } else { oo.reverse() }
                    });
                }
                let sep = render_elements(separator, params, pipes, io, context)?;
                let finalelems: Vec<String> = mapped.iter_mut().map(|ii| ii.1.clone()).collect();
                Ok(finalelems.join(&sep))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ForIterationType<'a>{
    Values(&'a TemplateValue),
    Filenames(&'a str),
    FileAt(&'a TemplateValue, String),
}

impl<'a> std::fmt::Display for ForIterationType<'a> {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForIterationType::FileAt(value, filename) => write!(ff, "file \"{}\" derived from \"{}\"", filename, value),
            ForIterationType::Filenames(filename) => write!(ff, "file \"{}\"", filename),
            ForIterationType::Values(value) => write!(ff, "\"{}\"", value),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ForIteration<'a>(YamlValue, ForIterationType<'a>, usize);

fn for_make_iterable<'a>(
    params: & YamlMap,
    groupings: &'a Vec<ForGrouping>,
    pipes: &'a PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext
) -> Result<Vec<ForIteration<'a>>, TemplateError> {
    let mut entries: Vec<ForIteration> = Vec::new();
    for gg in groupings {
        for vv in gg.values.iter() {
            let value = &vv.value;
            let lookup = lookup_value(&value, params)?;
            let piped = execute_pipes(&lookup, &vv.pipes, params, PipeInputSource::Value(&value), pipes, io, context)?;
            let location = ForIterationType::Values(&value);
            let as_vec = to_iterable(&location, &piped)?;
            for (ind, finalval) in as_vec.into_iter().enumerate() {
                entries.push(ForIteration(finalval, location.clone(), ind));
            }
        }
        for filename in gg.filenames.iter() {
            let lookup = io.read_yaml(&filename)
                .map_err(|xx| TemplateError::YamlFileError(xx))?;
            let location= ForIterationType::Filenames(&filename);
            let as_vec = to_iterable(&location, lookup)?;
            for (ind, finalval) in as_vec.into_iter().enumerate() {
                entries.push(ForIteration(finalval, location.clone(), ind));
            }
        }
        for fileat in gg.files_at.iter() {
            let lookup = lookup_value(&fileat, params)?;
            let filename = tostr(&lookup)?;
            let file = io.read_yaml(&filename)
                .map_err(|xx| TemplateError::YamlFileError(xx))?;
            let location = ForIterationType::FileAt(&fileat, filename.to_owned());
            let as_vec = to_iterable(&location, file)?;
            for (ind, finalval) in as_vec.into_iter().enumerate() {
                entries.push(ForIteration(finalval, location.clone(), ind));
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
