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

pub fn map_m<'a, A, B, E>(
    list: &'a Vec<A>,
    mut func: impl FnMut(&'a A) -> Result<B, E>
) -> Result<Vec<B>, E> {
    let mut current: Result<Vec<B>, E> = Ok(vec![]);
    for aa in list {
        current = match current {
            Err(..) => current,
            Ok(mut ss) => match func(aa) {
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
