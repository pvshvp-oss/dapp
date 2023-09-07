/// To be implemented by any path-like type to indicate whether it exists,
/// and to be able to determine what actions can be done with it.
/// The meanings of the named lifetimes, associate types, and generic types are
/// as follows:
/// - [`'a`]: The lifetime of an output `Path`.
/// - [`P`]: The path-like type on which the methods act.
/// - [`P1`]: The output reference to path-like object.
pub trait ValidPath<'a, P> {
    type P1;

    fn exists(&self) -> bool;

    fn is_readable(&self) -> bool;

    fn is_writable(&self) -> bool;

    fn is_executable(&self) -> bool;

    fn is_creatable(&self) -> bool;

    fn largest_valid_subset(&'a self) -> Option<Self::P1>;
}

/// Implement for types that can be converted to &Path
impl<'a, P> ValidPath<'a, P> for P
where
    P: AsRef<Path>,
{
    type P1 = &'a Path;

    fn exists(&self) -> bool {
        <Self as AsRef<Path>>::as_ref(self).exists()
    }

    fn is_readable(&self) -> bool {
        permissions::is_readable(self).unwrap_or(false)
    }

    fn is_writable(&self) -> bool {
        permissions::is_writable(self).unwrap_or(false)
    }

    fn is_executable(&self) -> bool {
        permissions::is_executable(self).unwrap_or(false)
    }

    fn is_creatable(&self) -> bool {
        // Declare a path as creatable if the largest existing subset of it
        // (i.e. the innermost existing file/parent in the path) is writable,
        // allowing one to create the rest of the path legally.
        match self.largest_valid_subset() {
            Some(p) => p.is_writable(),
            None => false,
        }
    }

    /// Find the innermost existing file/parent in the path
    fn largest_valid_subset(&'a self) -> Option<Self::P1> {
        let mut path = self.as_ref();
        while !path.exists() {
            match path.parent() {
                Some(p) => path = p,
                None => {
                    return None;
                }
            };
        }
        Some(path)
    }
}

/// Implement for types that can be converted to Option<&Path>
impl<'a, P> ValidPath<'a, P> for Option<P>
where
    P: AsRef<Path>,
{
    type P1 = &'a Path;

    fn exists(&self) -> bool {
        match self {
            Some(p) => p.exists(),
            None => false,
        }
    }

    fn is_readable(&self) -> bool {
        match self {
            Some(p) => p.is_readable(),
            None => false,
        }
    }

    fn is_writable(&self) -> bool {
        match self {
            Some(p) => p.is_writable(),
            None => false,
        }
    }

    fn is_creatable(&self) -> bool {
        match self {
            Some(p) => p.is_creatable(),
            None => false,
        }
    }

    fn is_executable(&self) -> bool {
        match self {
            Some(p) => p.is_executable(),
            None => false,
        }
    }

    fn largest_valid_subset(&'a self) -> Option<Self::P1> {
        match self {
            Some(p) => p.largest_valid_subset(),
            None => None,
        }
    }
}

/// To be implemented for an iterator of path-like objects.
/// The meanings of the named lifetimes, associate types, and generic types are
/// as follows:
/// - [`'a`]: The lifetime of an output `Path`.
/// - [`P`]: The path-like type on which the methods act.
/// - [`Q`]: The input path-like type. It could , for example, represent an
/// optional path-like object.
pub trait ValidPaths<'a, P, Q>
where
    P: AsRef<Path> + 'a,
{
    fn first_existing_path(&mut self) -> Option<P>;

    fn first_readable_path(&mut self) -> Option<P>;

    fn first_writable_path(&mut self) -> Option<P>;

    fn first_executable_path(&mut self) -> Option<P>;

    fn first_creatable_path(&mut self) -> Option<P>;

    fn all_existing_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a>;

    fn all_readable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a>;

    fn all_writable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a>;

    fn all_executable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a>;

    fn all_creatable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a>;

    fn first_valid_path(&'a mut self, f: fn(&Q) -> bool) -> Option<P>;

    fn all_valid_paths(&'a mut self, f: fn(&Q) -> bool) -> Box<dyn Iterator<Item = P> + 'a>;
}

/// Implement for iterators of objects that can be converted to &Path.
impl<'a, P, I> ValidPaths<'a, P, P> for I
where
    I: Iterator<Item = P> + 'a,
    P: AsRef<Path> + 'a,
{
    fn first_existing_path(&mut self) -> Option<P> {
        self.first_valid_path(P::exists)
    }

    fn first_readable_path(&mut self) -> Option<P> {
        self.first_valid_path(P::is_readable)
    }

    fn first_writable_path(&mut self) -> Option<P> {
        self.first_valid_path(P::is_writable)
    }

    fn first_executable_path(&mut self) -> Option<P> {
        self.first_valid_path(P::is_executable)
    }

    fn first_creatable_path(&mut self) -> Option<P> {
        self.first_valid_path(P::is_creatable)
    }

    fn all_existing_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(P::exists)
    }

    fn all_readable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(P::is_readable)
    }

    fn all_writable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(P::is_writable)
    }

    fn all_executable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(P::is_executable)
    }

    fn all_creatable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(P::is_creatable)
    }

    fn first_valid_path(&mut self, f: fn(&P) -> bool) -> Option<P> {
        self.find(|p| f(p))
    }

    fn all_valid_paths(&'a mut self, f: fn(&P) -> bool) -> Box<dyn Iterator<Item = P> + 'a> {
        Box::new(self.filter(move |p| f(p)))
    }
}

/// Implement for iterators of objects that can be converted to Option<&Path>.
impl<'a, P, I> ValidPaths<'a, P, Option<P>> for I
where
    I: Iterator<Item = Option<P>> + 'a,
    P: AsRef<Path> + 'a,
{
    fn first_existing_path(&mut self) -> Option<P> {
        self.first_valid_path(Option::<P>::exists)
    }

    fn first_readable_path(&mut self) -> Option<P> {
        self.first_valid_path(Option::<P>::is_readable)
    }

    fn first_writable_path(&mut self) -> Option<P> {
        self.first_valid_path(Option::<P>::is_writable)
    }

    fn first_executable_path(&mut self) -> Option<P> {
        self.first_valid_path(Option::<P>::is_executable)
    }

    fn first_creatable_path(&mut self) -> Option<P> {
        self.first_valid_path(Option::<P>::is_creatable)
    }

    fn all_existing_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(Option::<P>::exists)
    }

    fn all_readable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(Option::<P>::is_readable)
    }

    fn all_writable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(Option::<P>::is_writable)
    }

    fn all_executable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(Option::<P>::is_executable)
    }

    fn all_creatable_paths(&'a mut self) -> Box<dyn Iterator<Item = P> + 'a> {
        self.all_valid_paths(Option::<P>::is_creatable)
    }

    fn first_valid_path(&mut self, f: fn(&Option<P>) -> bool) -> Option<P> {
        self.find(|p| f(p))
            .flatten()
    }

    fn all_valid_paths(
        &'a mut self,
        f: fn(&Option<P>) -> bool,
    ) -> Box<dyn Iterator<Item = P> + 'a> {
        Box::new(
            self.filter(move |p| f(p))
                .flat_map(convert::identity),
        )
    }
}

// region: IMPORTS

use std::{convert, path::Path};

// endregion: IMPORTS
