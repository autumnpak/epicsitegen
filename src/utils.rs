pub enum MaybeRef<'a, T> {
    Owned(T),
    Ref(&'a T)
}

pub fn fold_m<'a, A, B, E>(
    start: &'a B,
    list: &'a Vec<A>,
    mut func: impl FnMut(&'a B, &'a A) -> Result<&'a B, E>
) -> Result<&'a B, E> {
    let mut current: Result<&B, E> = Ok(start);
    for aa in list {
        current = match current {
            Err(..) => current,
            Ok(ss) => func(ss, aa),
        }
    }
    current
}

pub fn fold_m_mut<A, B, E>(
    start: B,
    list: Vec<A>,
    mut func: impl FnMut(&B, A) -> Result<B, E>
) -> Result<B, E> {
    let mut current: Result<B, E> = Ok(start);
    for aa in list {
        current = match current {
            Err(..) => current,
            Ok(ss) => func(&ss, aa),
        }
    }
    current
}

pub fn map_m<'a, A, B, E>(
    list: Vec<A>,
    func: impl FnMut(A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    map_m_mut(list, func)
}

pub fn map_m_index<'a, A, B, E>(
    list: Vec<A>,
    func: impl FnMut(usize, A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    map_m_mut_index(list, func)
}

pub fn map_m_ref<'a, A, B, E>(
    list: &'a Vec<A>,
    mut func: impl FnMut(&'a A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    map_m_ref_index(list, |_, mm| func(mm))
}

pub fn map_m_ref_index<'a, A, B, E>(
    list: &'a Vec<A>,
    mut func: impl FnMut(usize, &'a A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    let mut current: Result<Vec<B>, E> = Ok(vec![]);
    for (ind, aa) in list.iter().enumerate() {
        current = match current {
            Err(..) => current,
            Ok(mut ss) => match func(ind, aa) {
                Err(ee) => Err(ee),
                Ok(ss2) => {
                    ss.push(ss2);
                    Ok(ss)
                }
            }
        }
    }
    current
}

pub fn map_m_mut_index<A, B, E>(
    list: Vec<A>,
    mut func: impl FnMut(usize, A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    let mut current: Result<Vec<B>, E> = Ok(vec![]);
    for (idx, aa) in list.into_iter().enumerate() {
        current = match current {
            Err(..) => current,
            Ok(mut ss) => match func(idx, aa) {
                Err(ee) => Err(ee),
                Ok(ss2) => {
                    ss.push(ss2);
                    Ok(ss)
                }
            }
        }
    }
    current
}

pub fn map_m_mut<A, B, E>(
    list: Vec<A>,
    mut func: impl FnMut(A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    map_m_mut_index(list, |_, mm| func(mm))
}

pub fn map_mut<A, B>(
    list: Vec<A>,
    mut func: impl FnMut(A) -> B
) -> Vec<B> {
    let mut entries = Vec::new();
    for ii in list { entries.push(func(ii)) }
    entries
}
