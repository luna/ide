use crate::prelude::*;

use crate::dirty;
use crate::data::function::callback::*;
use crate::display::symbol::attribute as attr;
use crate::display::symbol::attribute::IsAttribute;
use crate::display::symbol::attribute::Shape;
use crate::display::symbol::attribute::SharedAttribute;
use crate::display::symbol::mesh;
use crate::system::web::fmt;
use crate::system::web::group;
use crate::system::web::Logger;
use crate::closure;
use crate::data::opt_vec::OptVec;



use rustc_hash::FxHashMap;
use std::collections::hash_map;
use std::cell;


// // ===============
// // === WeakSet ===
// // ===============

// // === Definition ===

// pub type Key       = usize;
// pub type Value <T> = Weak<ValueGuard<T>>;
// pub type Set   <T> = FxHashMap<Key, Value<T>>;

// #[derive(Shrinkwrap)]
// #[derive(Debug)]
// pub struct WeakSet<T> { 
//     data: Rc<RefCell<Set<T>>> 
// }

// impl<T> WeakSet<T> {
//     pub fn key_of(t: &Rc<ValueGuard<T>>) -> usize {
//         let t = Rc::downgrade(t);
//         t.as_raw() as usize
//     }

//     pub fn key_of_weak(t: &Weak<ValueGuard<T>>) -> usize {
//         t.as_raw() as usize
//     }

//     pub fn clone_ref(&self) -> Self {
//         let data = Rc::clone(&self.data);
//         Self { data }
//     }

//     pub fn len(&self) -> usize {
//         self.borrow().len()
//     }

//     pub fn iter(&self) -> Iter<T> {
//         let borrow = self.borrow();
//         let values = borrow.values();
//         let values = unsafe { Self::cast_values_lifetime(values) };
//         Iter { values, borrow }
//     }

//     pub fn insert(&self, t: &Rc<ValueGuard<T>>) {
//         let val = Rc::downgrade(t);
//         let key = Self::key_of_weak(&val);
//         self.data.borrow_mut().insert(key, val);
//     }

//     pub fn rc(&self, elem:T) -> Rc<ValueGuard<T>> {
//         let set   = self.clone_ref();
//         let guard = ValueGuard { elem, set };
//         let rc    = Rc::new(guard);
//         self.insert(&rc);
//         rc
//     }

//     unsafe fn cast_values_lifetime<'t1, 't2, A, B>
//     (t: hash_map::Values<'t1, A, B>) -> hash_map::Values<'t2, A, B> { 
//         std::mem::transmute(t) 
//     }
// }

// impl<T> Default for WeakSet<T> {
//     fn default() -> Self {
//         let data = Rc::new(RefCell::new(default()));
//         Self { data }
//     }
// }

// impl<'t, T> IntoIterator for &'t WeakSet<T> {
//     type Item     = Rc<ValueGuard<T>>;
//     type IntoIter = Iter<'t, T> ;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter()
//     }
// }

// // === ValueGuard ===

// #[derive(Debug)]
// pub struct ValueGuard<T> {
//     pub elem : T,
//     pub set  : WeakSet<T>,  
// }

// impl<T> Deref for ValueGuard<T> {
//     type Target = T;
//     fn deref(&self) -> &Self::Target {
//         &self.elem
//     }
// }

// impl<T> DerefMut for ValueGuard<T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.elem
//     }
// }

// impl<T> Drop for ValueGuard<T> {
//     fn drop(&mut self) {
//         let key = self as *const ValueGuard<T> as usize;
//         self.set.borrow_mut().remove(&key);
//     }
// }

// // === Iter ===

// pub struct Iter<'t, T> {
//     values: hash_map::Values<'t, Key, Value<T>>,
//     borrow: cell::Ref<'t, Set<T>>
// }

// impl<'t, T> Deref for Iter<'t, T> {
//     type Target = hash_map::Values<'t, Key, Value<T>>;
//     fn deref(&self) -> &Self::Target {
//         &self.values
//     }
// }

// impl<'t, T> DerefMut for Iter<'t, T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.values
//     }
// }

// impl<'t, T> Iterator for Iter<'t, T> {
//     type Item =  Rc<ValueGuard<T>>;

//     fn next(&mut self) -> Option<(Self::Item)> {
//         self.values.next().and_then(|t| t.upgrade())
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         self.values.size_hint()
//     }
// }

// // =============
// // === Tests ===
// // =============

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_weak_set() {
//         let set: WeakSet<String> = default();
//         assert_eq!(set.len(), 0);
//         let s1 = set.rc("s1".to_string());
//         let s2 = set.rc("s3".to_string());
//         let s3 = set.rc("s2".to_string());
//         assert_eq!(set.len(), 3);
//         {
//             let st1 = set.rc("s1".to_string());
//             assert_eq!(set.len(), 4);
//         }
//         assert_eq!(set.len(), 3);
//     }
// }


// ===============
// === WeakMap ===
// ===============

// === Definition ===

pub type WeakRef <Key, Val> = Weak<ValueGuard<Key, Val>>;
pub type Map     <Key, Val> = FxHashMap<Key, WeakRef<Key, Val>>;

pub trait KeyCtx = Copy + Eq + std::hash::Hash;

#[derive(Shrinkwrap)]
#[derive(Derivative)]
#[derivative(Debug(bound="Key: KeyCtx + Debug, Val: Debug"))]
pub struct WeakMap<Key: KeyCtx, Val> { 
    data: Rc<RefCell<Map<Key, Val>>> 
}

impl<Key: KeyCtx, Val> 
WeakMap<Key, Val> {
    pub fn key_of(t: &Rc<ValueGuard<Key, Val>>) -> usize {
        let t = Rc::downgrade(t);
        t.as_raw() as usize
    }

    pub fn key_of_weak(t: &Weak<ValueGuard<Key, Val>>) -> usize {
        t.as_raw() as usize
    }

    pub fn clone_ref(&self) -> Self {
        let data = Rc::clone(&self.data);
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.borrow().len()
    }

    pub fn iter(&self) -> Iter<Key, Val> {
        let borrow = self.borrow();
        let values = borrow.values();
        let values = unsafe { Self::cast_values_lifetime(values) };
        Iter { values, borrow }
    }

    pub fn insert(&self, key:Key, t: &Rc<ValueGuard<Key, Val>>) {
        let val = Rc::downgrade(t);
        self.data.borrow_mut().insert(key, val);
    }

    pub fn rc(&self, key:Key, val:Val) -> Rc<ValueGuard<Key, Val>> {
        let map   = self.clone_ref();
        let guard = ValueGuard { key, val, map };
        let rc    = Rc::new(guard);
        self.insert(key, &rc);
        rc
    }

    unsafe fn cast_values_lifetime<'t1, 't2, A, B>
    (t: hash_map::Values<'t1, A, B>) -> hash_map::Values<'t2, A, B> { 
        std::mem::transmute(t) 
    }
}

impl<Key: KeyCtx, Val> Default for WeakMap<Key, Val> {
    fn default() -> Self {
        let data = Rc::new(RefCell::new(default()));
        Self { data }
    }
}

impl<'t, Key: KeyCtx, Val> IntoIterator for &'t WeakMap<Key, Val> {
    type Item     = Rc<ValueGuard<Key, Val>>;
    type IntoIter = Iter<'t, Key, Val> ;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// === ValueGuard ===

#[derive(Derivative)]
#[derivative(Debug(bound="Key: KeyCtx + Debug, Val: Debug"))]
pub struct ValueGuard<Key: KeyCtx, Val> {
    pub key : Key,
    pub val : Val,
    pub map : WeakMap<Key, Val>,  
}

impl<Key:KeyCtx, Val> Deref for ValueGuard<Key, Val> {
    type Target = Val;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<Key:KeyCtx, Val> DerefMut for ValueGuard<Key, Val> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

impl<Key:KeyCtx, Val> Drop for ValueGuard<Key, Val> {
    fn drop(&mut self) {
        self.map.borrow_mut().remove(&self.key);
    }
}

// === Iter ===

pub struct Iter<'t, Key: KeyCtx, Val> {
    values: hash_map::Values<'t, Key, WeakRef<Key, Val>>,
    borrow: cell::Ref<'t, Map<Key, Val>>
}

impl<'t, Key: KeyCtx, Val> Deref for Iter<'t, Key, Val> {
    type Target = hash_map::Values<'t, Key, WeakRef<Key, Val>>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<'t, Key: KeyCtx, Val> DerefMut for Iter<'t, Key, Val> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<'t, Key: KeyCtx, Val> Iterator for Iter<'t, Key, Val> {
    type Item =  Rc<ValueGuard<Key, Val>>;

    fn next(&mut self) -> Option<(Self::Item)> {
        self.values.next().and_then(|t| t.upgrade())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.values.size_hint()
    }
}

// =============
// === Tests ===
// =============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weak_set() {
        let set: WeakMap<String> = default();
        assert_eq!(set.len(), 0);
        let s1 = set.rc("s1".to_string());
        let s2 = set.rc("s3".to_string());
        let s3 = set.rc("s2".to_string());
        assert_eq!(set.len(), 3);
        {
            let st1 = set.rc("s1".to_string());
            assert_eq!(set.len(), 4);
        }
        assert_eq!(set.len(), 3);
    }
}

// ============
// === Pool ===
// ============

// === Definition ===

#[derive(Debug, Default)]
pub struct Pool<Item> {
    free: Rc<RefCell<Vec<Item>>>
}

type IxPool = Pool<usize>;

impl<Item> 
Pool<Item> {
    pub fn pop(&self) -> Option<Item> {
        self.free.borrow_mut().pop()
    }

    pub fn push(&self, item: Item) {
        self.free.borrow_mut().push(item)
    }

    pub fn clone_ref(&self) -> Self {
        Self { free: Rc::clone(&self.free) }
    }
}

// =================
// === PoolGuard ===
// =================

// === Definition ===

#[derive(Debug)]
pub struct PoolGuard<Item: Copy, T> {
    pub elem   : T,
    pub item   : Item,
    pub pool   : Pool<Item>
}

type IxPoolGuard<T> = PoolGuard<usize, T>;

// === Instances ===

impl<Item: Copy, T> 
Deref for PoolGuard<Item, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.elem
    }
}

impl<Item: Copy, T> 
DerefMut for PoolGuard<Item, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elem
    }
}

impl<Item: Copy, T> 
Drop for PoolGuard<Item, T> {
    fn drop(&mut self) {
        self.pool.push(self.item);
    }
}

// // ====================
// // === MeshRegistry ===
// // ====================

// // === Definition ===

// #[derive(Derivative)]
// #[derivative(Debug(bound=""))]
// pub struct MeshRegistry <OnDirty> {
//     pub meshes     : WeakMap<MeshID, Mesh<OnDirty>>,
//     pub mesh_dirty : MeshDirty<OnDirty>,
//     pub ix_pool    : Vec<MeshID>,
//     pub logger     : Logger,
// }

// // === Types ===

// pub type MeshID           = usize;
// pub type Ref       <T>       = Rc<ValueGuard<MeshID, T>>;
// pub type MeshDirty <OnDirty> = dirty::SharedSet<MeshID, OnDirty>;
// pub type Mesh      <OnDirty> = mesh::SharedMesh<Closure_mesh_on_dirty<OnDirty>>;

// // === Callbacks ===

// closure!(mesh_on_dirty<Callback: Callback0>
//     (dirty: MeshDirty<Callback>, ix: MeshID) || { dirty.set(ix) });

// // === Implementation ===

// impl<OnDirty: Callback0> MeshRegistry<OnDirty> {
//     pub fn new(logger: Logger, on_dirty: OnDirty) -> Self {
//         logger.info("Initializing.");
//         let mesh_logger = logger.sub("mesh_dirty");
//         let mesh_dirty  = MeshDirty::new(on_dirty, mesh_logger);
//         let meshes      = default();
//         let ix_pool     = default();
//         Self { meshes, mesh_dirty, ix_pool, logger }
//     }

//     pub fn new_mesh(&mut self) -> Ref<Mesh<OnDirty>> {
//         let opt_ix     = self.ix_pool.pop();
//         let ix         = opt_ix.unwrap_or_else(|| self.meshes.len());
//         let reused     = opt_ix.is_some(); 

//         let mesh_dirty = self.mesh_dirty.clone();
//         let on_dirty   = mesh_on_dirty(mesh_dirty, ix);
//         let logger     = self.logger.sub(format!("mesh{}",ix));
//         let mesh       = Mesh::new(logger, on_dirty);
//         self.meshes.rc(ix, mesh)
//     }

//     pub fn update(&self) {
//         for mesh in self.meshes.iter() {
//         }
//     }
// }

// ====================
// === MeshRegistry ===
// ====================

// === Definition ===

#[derive(Derivative)]
#[derivative(Debug(bound=""))]
pub struct MeshRegistry <OnDirty> {
    pub meshes     : OptVec<Mesh<OnDirty>>,
    pub mesh_dirty : MeshDirty<OnDirty>,
    pub logger     : Logger,
}

// === Types ===

pub type MeshID              = usize;
pub type MeshDirty <OnDirty> = dirty::SharedSet<MeshID, OnDirty>;

pub type Mesh           <OnDirty> = mesh::Mesh           <Closure_mesh_on_dirty<OnDirty>>;
pub type Geometry       <OnDirty> = mesh::Geometry       <Closure_mesh_on_dirty<OnDirty>>;
pub type Scopes         <OnDirty> = mesh::Scopes         <Closure_mesh_on_dirty<OnDirty>>;
pub type AttributeScope <OnDirty> = mesh::AttributeScope <Closure_mesh_on_dirty<OnDirty>>;
pub type UniformScope   <OnDirty> = mesh::UniformScope   <Closure_mesh_on_dirty<OnDirty>>;
pub type GlobalScope    <OnDirty> = mesh::GlobalScope    <Closure_mesh_on_dirty<OnDirty>>;
pub type Attribute <T, OnDirty> = mesh::Attribute <T, Closure_mesh_on_dirty<OnDirty>>;

// === Callbacks ===

closure!(mesh_on_dirty<Callback: Callback0>
    (dirty: MeshDirty<Callback>, ix: MeshID) || { dirty.set(ix) });

// === Implementation ===

impl<OnDirty: Callback0> MeshRegistry<OnDirty> {
    pub fn new(logger: Logger, on_dirty: OnDirty) -> Self {
        logger.info("Initializing.");
        let mesh_logger = logger.sub("mesh_dirty");
        let mesh_dirty  = MeshDirty::new(on_dirty, mesh_logger);
        let meshes      = default();
        Self { meshes, mesh_dirty, logger }
    }

    pub fn new_mesh(&mut self) -> MeshID {
        let mesh_dirty = self.mesh_dirty.clone();
        let logger     = &self.logger;
        self.meshes.insert_with_ix(|ix| {
            let on_dirty   = mesh_on_dirty(mesh_dirty, ix);
            let logger     = logger.sub(format!("mesh{}",ix));
            Mesh::new(logger, on_dirty)
        })
    }

    // pub fn update(&self) {
    //     for mesh in self.meshes.iter() {
    //     }
    // }
}

impl<OnDirty> Index<usize> for MeshRegistry<OnDirty> {
    type Output = Mesh<OnDirty>;
    fn index(&self, ix: usize) -> &Self::Output {
        self.meshes.index(ix)
    }
}

impl<OnDirty> IndexMut<usize> for MeshRegistry<OnDirty> {
    fn index_mut(&mut self, ix: usize) -> &mut Self::Output {
        self.meshes.index_mut(ix)
    }
}

