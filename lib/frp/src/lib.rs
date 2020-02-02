//! This module implements an Functional Reactive Programming system. It is an advanced event
//! handling framework which allows describing events and actions by creating declarative event
//! flow diagrams.

#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]

#![feature(specialization)]
#![feature(trait_alias)]
#![feature(weak_into_raw)]
#![feature(associated_type_defaults)]

pub mod debug;

use basegl_prelude    as prelude;
use basegl_system_web as web;

use crate::prelude::*;

use debug::*;
use percent_encoding;
use std::borrow::Cow;



// ==============
// === Macros ===
// ==============

macro_rules! alias {
    ($( $(#$meta:tt)* $name:ident = {$($tok:tt)*} )*) => {$(
        $(#$meta)*
        pub trait $name: $($tok)* {}
        impl<T:$($tok)*> $name for T {}
    )*}
}








// ===============
// === Message ===
// ===============

// === Types ===

alias! {
    /// Message is a data send between FRP nodes.
    /// There are two important message implementation – the `BehaviorMessage` and `EventMessage`.
    Message = { MessageValue + ValueWrapper + KnownDynNode + PhantomInto<MessageType> }

    /// Abstraction for a value carried by a message.
    MessageValue = { Clone + Debug + Default + 'static }
}

/// Accessor to a value of a given message. For example, `Value<Behavior<i32>>` resolves to `i32`.
pub type Value<T> = Content<T>;

/// Alias to `Wrapper` with the inner type being `Debug`.
pub trait ValueWrapper = Wrapper where Content<Self>:Debug;


// === Definition ===

#[derive(Clone,Debug,Copy)]
pub enum MessageType {Event,Behavior}

impl<T> From<PhantomData<EventMessage<T>>> for MessageType {
    fn from(_:PhantomData<EventMessage<T>>) -> Self {
        Self::Event
    }
}

impl<T> From<PhantomData<BehaviorMessage<T>>> for MessageType {
    fn from(_:PhantomData<BehaviorMessage<T>>) -> Self {
        Self::Behavior
    }
}

/// A newtype containing a value of an event.
#[derive(Clone,Copy,Debug,Default)]
pub struct EventMessage<T>(pub T);

/// A newtype containing a value of a behavior.
#[derive(Clone,Copy,Debug,Default)]
pub struct BehaviorMessage<T>(pub T);


// === API ===

impl<T:Clone> EventMessage<T> {
    /// Get the unwrapped value of this message.
    pub fn value(&self) -> T {
        self.unwrap().clone()
    }
}

impl<T:Clone> BehaviorMessage<T> {
    /// Get the unwrapped value of this message.
    pub fn value(&self) -> T {
        self.unwrap().clone()
    }
}


// === Wrappers ===

impl<T> HasContent for EventMessage<T> { type Content = T; }
impl<T> Wrap       for EventMessage<T> { fn wrap   (t:T)   -> Self { EventMessage(t) } }
impl<T> Unwrap     for EventMessage<T> { fn unwrap (&self) -> &T   { &self.0 } }

impl<T> HasContent for BehaviorMessage<T> { type Content = T; }
impl<T> Wrap       for BehaviorMessage<T> { fn wrap   (t:T)   -> Self { BehaviorMessage(t) } }
impl<T> Unwrap     for BehaviorMessage<T> { fn unwrap (&self) -> &T   { &self.0 } }





// ======================
// === Input / Output ===
// ======================

/// Event input associated type. Please note that FRP nodes can have maximum one event input.
/// In such a case this trait points to it.
pub trait KnownEventInput {
    /// The event input type.
    type EventInput : Message;
}

/// Event input accessor.
pub type EventInput<T> = <T as KnownEventInput>::EventInput;


/// Each FRP node has a single node, which type is described by this trait.
pub trait KnownOutput {
    /// The output type.
    type Output : Message;
}

pub trait KnownOutputType {
    fn output_type(&self) -> MessageType;
    fn output_type_value_name(&self) -> String;
}

impl<T:KnownOutput> KnownOutputType for T
    where Output<Self> : Message {
    fn output_type(&self) -> MessageType {
        PhantomData::<Output<Self>>.into()
    }

    fn output_type_value_name(&self) -> String {
        let qual_name = type_name::<Output<Self>>();
        let param     = qual_name.split('<').skip(1).collect::<String>();
        let param     = &param[0..param.len()-1];
        let param     = param.rsplit("::").collect::<Vec<_>>()[0];
        param.into()
    }
}

/// Node output accessor.
pub type Output<T> = <T as KnownOutput>::Output;



pub trait HasEventTargets : KnownOutput {
    fn add_event_target(&self, target:AnyEventConsumer<Output<Self>>);
}

pub trait HasCurrentValue : KnownOutput {
    fn current_value(&self) -> Value<Output<Self>>;
}



// ===============
// === DynNode ===
// ===============

alias! {
    /// Bounds required for every node storage type.
    DynNodeBounds = {
        Debug + GraphvizBuilder + HasId + HasDisplayId + HasInputs + HasLabel + KnownOutput
    }
}

/// Type level abstraction for node internal storage. The internal storage differs for events and
/// behaviors, as they provide different functionality. For example, behaviors allow lookup for the
/// current value, which does not make sense in case of events.
pub trait KnownDynNode {
    /// The node storage type.
    type DynNode: DynNodeBounds + CloneRef;
}

/// Internal node storage type accessor.
pub type DynNode<T> = <T as KnownDynNode>::DynNode;


// === EventDynNodeBounds ===

/// Newtype wrapper for any event node storage.
#[derive(Debug,Derivative,Shrinkwrap)]
#[derivative(Clone(bound=""))]
pub struct EventDynNode<Out> {
    rc: Rc<dyn EventDynNodeBounds<Output=EventMessage<Out>>>,
}

alias! {
    /// Bounds for any event node storage.
    EventDynNodeBounds = { DynNodeBounds + HasEventTargets + EventEmitter }
}

impl<Out:MessageValue> KnownDynNode for EventMessage<Out> {
    type DynNode = EventDynNode<Out>;
}

impl<Out> Unwrap     for EventDynNode<Out> {}
impl<Out> CloneRef   for EventDynNode<Out> {}
impl<Out> HasContent for EventDynNode<Out> {
    // TODO: Simplify after fixing https://github.com/rust-lang/rust/issues/68776
    type Content = <EventDynNode<Out> as Deref>::Target;
}
impl<Out:MessageValue> KnownOutput for EventDynNode<Out> {
    type Output = EventMessage<Out>;
}


// === BehaviorDynNodeBounds ===

/// Newtype wrapper for any behavior node storage.
#[derive(Debug,Derivative,Shrinkwrap)]
#[derivative(Clone(bound=""))]
pub struct BehaviorDynNode<Out> {
    rc: Rc<dyn BehaviorDynNodeBounds<Output=BehaviorMessage<Out>>>,
}

alias! {
    /// Bounds for any behavior node storage.
    BehaviorDynNodeBounds = { DynNodeBounds + HasCurrentValue  }
}

impl<Out:MessageValue> KnownDynNode for BehaviorMessage<Out> {
    type DynNode = BehaviorDynNode<Out>;
}

impl<Out> Unwrap     for BehaviorDynNode<Out> {}
impl<Out> CloneRef   for BehaviorDynNode<Out> {}
impl<Out> HasContent for BehaviorDynNode<Out> {
    // TODO: Simplify after fixing https://github.com/rust-lang/rust/issues/68776
    type Content = <BehaviorDynNode<Out> as Deref>::Target;
}
impl<Out:MessageValue> KnownOutput for BehaviorDynNode<Out> {
    type Output = BehaviorMessage<Out>;
}







// =============
// === Label ===
// =============

/// Abstraction for labeled entities. Used mainly for debugging purposes.
pub trait HasLabel {
    /// Label of the entity.
    fn label(&self) -> CowString;
}

impl<T:Unwrap> HasLabel for T
where Content<T> : HasLabel {
    default fn label(&self) -> CowString {
        self.unwrap().label()
    }
}



// ============
// === Node ===
// ============

// === Types ===

/// The type of any FRP node which produces event messages. Having a reference to a node is like
/// having a reference to network endpoint which transmits messages of a given type. Thus, it is a
/// nice mental simplification to think about it just like about an event (stream).
pub type Event<T> = Node<EventMessage<T>>;

/// The type of any FRP node which can be queried for behavior value. Having a reference to a node
/// is like having a reference to network endpoint which transmits messages of a given type. Thus,
/// it is a nice mental simplification to think about it just like about a behavior.
pub type Behavior <T> = Node<BehaviorMessage<T>>;


// === Definition ===

/// Node is used as a common types for frp operations. For example, `Event<T>` is just an alias to
/// `Node<EventMessage<T>>`.
#[derive(Derivative)]
#[derivative(Clone(bound=""))]
#[derivative(Debug(bound=""))]
pub struct Node<Out:KnownDynNode> {
    storage: DynNode<Out>,
}

impl<Out:Message> Node<Out> {
    /// Constructor.
    pub fn new(storage:DynNode<Out>) -> Self {
        Self {storage}
    }
}


// === Instances ===

impl<Out:Message> KnownOutput for Node<Out> {
    type Output = Out;
}

impl<Out:Message> Deref for Node<Out> {
    type Target = DynNode<Out>;
    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<Out:Message> HasContent for Node<Out> {
    type Content = DynNode<Out>;
}

impl<Out:Message> Unwrap for Node<Out> {}



impl<Out:Message> CloneRef for Node<Out> {
    fn clone_ref(&self) -> Self {
        let storage = self.storage.clone_ref();
        Self {storage}
    }
}

impl<Out:Message> From<&Node<Out>> for Node<Out> {
    fn from(t:&Node<Out>) -> Self {
        t.clone_ref()
    }
}


// === Construction ===

impl<Storage,Out> From<&Storage> for Node<BehaviorMessage<Out>>
    where Storage : BehaviorDynNodeBounds<Output=BehaviorMessage<Out>> + Clone + 'static,
          Out     : MessageValue {
    fn from(storage:&Storage) -> Self {
        Self::new(BehaviorDynNode{rc:Rc::new(storage.clone())})
    }
}


impl<Storage,Out> From<&Storage> for Node<EventMessage<Out>>
    where Storage : EventDynNodeBounds<Output=EventMessage<Out>> + Clone + 'static,
          Out     : MessageValue {
    fn from(storage:&Storage) -> Self {
        Self::new(EventDynNode{rc:Rc::new(storage.clone())})
    }
}


// === AddTarget ===

/// Abstraction for adding a target to a given node. Nodes which carry behaviors do not need to
/// perform any operation here, while event streams want to register the nodes they want to send
/// notifications to.
pub trait AddTarget<T> {
    /// Adds a node as a target of the current flow.
    fn add_target(&self,t:&T);
}

impl<S,T:MessageValue> AddTarget<S> for Node<EventMessage<T>>
    where for<'t> &'t S : Into<AnyEventConsumer<EventMessage<T>>> {
    fn add_target(&self,t:&S) {
        self.add_event_target(t.into())
    }
}

impl<S,T:MessageValue> AddTarget<S> for Node<BehaviorMessage<T>> {
    fn add_target(&self,_:&S) {}
}

impl<Out:Message + KnownDynNode> AnyNodeOps for Node<Out> {}




impl<T:MessageValue> EventEmitter for Node<EventMessage<T>> {
    fn emit(&self, event:&Self::Output) {
        self.storage.emit(event)
    }
}




// =============
// === HasId ===
// =============

/// Each FRP node is assigned with an unique ID. This is currently used mainly for debugging
/// purposes.
pub trait HasId {
    /// Id of the entity.
    fn id(&self) -> usize;
}

impl<T:Unwrap> HasId for T where
Content<T> : HasId {
    default fn id(&self) -> usize {
        self.unwrap().id()
    }
}

/// Each FRP node can also be assigned with a `display_id`. Unlike `id`, the `display_id` does not
/// have to be unique. Nodes with the same `display_id` are displayed as a single node in the graph
/// view. Note that `display_id` defaults to `id` if not set explicitly to other value.
pub trait HasDisplayId {
    /// Getter.
    fn display_id(&self) -> usize;
    /// Setter.
    fn set_display_id(&self, id:usize);
}

impl<T> HasDisplayId for T
where T:Unwrap, Content<T> : HasDisplayId {
    default fn display_id(&self) -> usize {
        self.unwrap().display_id()
    }

    default fn set_display_id(&self, id:usize) {
        self.unwrap().set_display_id(id)
    }
}



pub trait AnyNodeOps : Debug + GraphvizBuilder + HasId + HasDisplayId + KnownOutputType {}

#[derive(Debug,Shrinkwrap)]
pub struct AnyNode {
    rc: Rc<dyn AnyNodeOps>,
}

impl<Out:Message+KnownDynNode+'static> From<&Node<Out>> for AnyNode {
    fn from(t:&Node<Out>) -> Self {
        t.clone().into()
    }
}

impl<T:AnyNodeOps+'static> From<T> for AnyNode {
    fn from(t:T) -> Self {
        let rc = Rc::new(t);
        Self {rc}
    }
}

pub trait HasInputs {
    fn inputs(&self) -> Vec<AnyNode>;
}

impl<T> HasInputs for T
where T:Unwrap, Content<T> : HasInputs {
    fn inputs(&self) -> Vec<AnyNode> {
        self.unwrap().inputs()
    }
}


impl KnownOutputType for AnyNode {
    fn output_type(&self) -> MessageType {
        self.rc.output_type()
    }

    fn output_type_value_name(&self) -> String {
        self.rc.output_type_value_name()
    }
}




// ===================
// === NodeWrapper ===
// ===================

// === NodeWrapper ===

/// `NodeWrapper` is an outer layer for every FRP node. For example, the `Source<Out>` node is just
/// an alias to `NodeWrapper<SourceShape<Out>>`, where `SourceShape` is it's internal representation.
/// This struct bundles each node with information about target edges. Although the edges are used
/// only to send events, they are bundled to every node type in order to keep the implementation
/// simple.
pub type NodeWrapper<Shape> = NodeWrapperTemplate<Shape,Output<Shape>>;

impl<Shape:KnownOutput> NodeWrapper<Shape> {
    /// Constructor.
    pub fn construct<Label>(label:Label, shape:Shape) -> Self
        where Label : Into<CowString> {
        let data = NodeWrapperTemplateData::construct(label,shape);
        let rc   = Rc::new(RefCell::new(data));
        let this = Self {rc};
        this.set_display_id(this.id());
        this
    }
}

impl<Shape,Out> NodeWrapperTemplate<Shape,Out> {
    /// Sends an event to all the children.
    pub fn emit_event(&self, event:&Out) {
        self.rc.borrow().targets.iter().for_each(|target| {
            target.on_event(event)
        })
    }
}

impl<Shape,T:MessageValue>
HasEventTargets for NodeWrapperTemplate<Shape,EventMessage<T>> {
    fn add_event_target(&self, target:AnyEventConsumer<EventMessage<T>>) {
        self.rc.borrow_mut().targets.push(target);
    }
}

//impl<Shape,T:MessageValue> EventEmitter for NodeWrapperTemplate<Shape,EventMessage<T>> {


//impl<Shape:BehaviorDynNodeBounds + Debug>
//BehaviorDynNodeBounds for NodeWrapper<Shape>
//where Output<Shape>:Message {
//    fn current_value(&self) -> Value<Output<Self>> {
//        self.rc.borrow().shape.current_value()
//    }
//}


// === NodeWrapperTemplate ===

/// Internal representation for `NodeWrapper`.
#[derive(Debug,Derivative)]
#[derivative(Default(bound="Shape:Default"))]
#[derivative(Clone(bound=""))]
pub struct NodeWrapperTemplate<Shape,Out> {
    rc: Rc<RefCell<NodeWrapperTemplateData<Shape,Out>>>
}

impl<Shape,Out>
HasId for NodeWrapperTemplate<Shape,Out> {
    fn id(&self) -> usize {
        Rc::downgrade(&self.rc).as_raw() as *const() as usize
    }
}

impl<Shape,Out>
HasDisplayId for NodeWrapperTemplate<Shape,Out> {
    fn display_id(&self) -> usize {
        self.rc.borrow().display_id
    }

    fn set_display_id(&self, id:usize) {
        self.rc.borrow_mut().display_id = id;
    }
}


impl<Shape,Out:Message>
KnownOutput for NodeWrapperTemplate<Shape,Out> {
    type Output = Out;
}

impl<Shape:KnownEventInput,Out>
KnownEventInput for NodeWrapperTemplate<Shape,Out>
    where EventInput<Shape> : Message {
    type EventInput = EventInput<Shape>;
}

impl<Shape,Out> CloneRef for NodeWrapperTemplate<Shape,Out> {}

impl<Shape:HasInputs,Out> GraphvizBuilder for NodeWrapperTemplate<Shape,Out> {
    fn graphviz_build(&self, builder:&mut Graphviz) {
        let type_name  = base_type_name::<Shape>();
        let label      = &self.rc.borrow().label;
        let id         = self.id();
        let display_id = self.display_id();
        if !builder.contains(id) {
            builder.add_node(id,display_id,type_name,label);
            for input in &self.rc.borrow().shape.inputs() {
                let input_id         = input.id();
                let input_display_id = input.display_id();
                let is_redirect      = input_id != input_display_id;
                input.graphviz_build(builder);
                builder.add_link(input_display_id,display_id,input.output_type(),&input.output_type_value_name());
            }
        }
    }
}

impl<Shape,T:MessageValue> EventEmitter for NodeWrapperTemplate<Shape,EventMessage<T>> {
    fn emit(&self, event:&Self::Output) {
        self.emit_event(event);
    }
}


fn base_type_name<T>() -> String {
    let qual_name = type_name::<T>();
    let base_name = qual_name.split("<").collect::<Vec<_>>()[0];
    let name      = base_name.rsplit("::").collect::<Vec<_>>()[0];
    let name      = name.split("Shape").collect::<Vec<_>>()[0];
    name.into()
}

impl<Shape:HasInputs,Out> HasInputs for NodeWrapperTemplate<Shape,Out> {
    fn inputs(&self) -> Vec<AnyNode> {
        self.rc.borrow().shape.inputs()
    }
}


impl<Shape,Out> HasLabel for NodeWrapperTemplate<Shape,Out> {
    fn label(&self) -> CowString {
        self.rc.borrow().label.clone()
    }
}


// === NodeWrapperTemplateData ===

/// Internal representation for `NodeWrapperTemplate`.
#[derive(Debug,Derivative)]
#[derivative(Default(bound="Shape:Default"))]
pub struct NodeWrapperTemplateData<Shape,Out> {
    label      : CowString,
    display_id : usize,
    shape      : Shape,
    targets    : Vec<AnyEventConsumer<Out>>,
}

impl<Shape,Out> NodeWrapperTemplateData<Shape,Out> {
    /// Constructor.
    pub fn construct<Label>(label:Label, shape:Shape) -> Self
        where Label : Into<CowString> {
        let label      = label.into();
        let targets    = default();
        let display_id = 0;
        Self {label,display_id,shape,targets}
    }
}



// =====================
// === EventConsumer ===
// =====================

// === Definition ===

/// Abstraction for nodes which are able to consume events.
pub trait EventConsumer: KnownEventInput + Debug {
    /// Function called on every new received event.
    fn on_event(&self, input:&Self::EventInput);
}


// === AnyEventConsumer ===

/// Abstraction for any node which consumes events of a given type.
#[derive(Clone,Debug,Shrinkwrap)]
pub struct AnyEventConsumer<In> {
    raw: Rc<dyn EventConsumer<EventInput=In>>,
}

impl<In:Message> AnyEventConsumer<In> {
    /// Constructor.
    pub fn new<A:EventConsumer<EventInput=In>+'static>(a:A) -> Self {
        let raw = Rc::new(a);
        Self {raw}
    }
}

impl<T,In> From<&T> for AnyEventConsumer<In>
    where T  : EventConsumer<EventInput=In> + Clone + 'static,
          In : Message {
    fn from(t:&T) -> Self {
        Self::new(t.clone())
    }
}


// =====================
// === EventEmitter ===
// =====================

// === Definition ===

/// Abstraction for nodes which are able to consume events.
pub trait EventEmitter: KnownOutput {
    /// Function called on every new received event.
    fn emit(&self, event:&Self::Output);
}



// =========================
// === Inference Helpers ===
// =========================

/// Message product type-level inference guidance.
pub trait Infer<T> {
    /// Inference results.
    type Result;
}

/// Accessor for inferred type.
pub type Inferred<T,X> = <X as Infer<T>>::Result;


// === Rules ===

macro_rules! inference_rules {
    ($( $pat:tt => $result:ident )*) => {$(
        inference_rule! { $pat => $result }
    )*}

}

macro_rules! inference_rule {
    ( $t1:ident => $result:ident ) => {
        impl<X,T1> Infer <$t1<T1>> for X { type Result = $result<X>; }
    };

    ( ($t1:ident) => $result:ident ) => {
        impl<X,T1> Infer <$t1<T1>> for X { type Result = $result<X>; }
    };

    ( ($t1:ident, $t2:ident) => $result:ident ) => {
        impl<X,T1,T2> Infer <($t1<T1>,$t2<T2>)> for X { type Result = $result<X>; }
    };

    ( ($t1:ident, $t2:ident, $t3:ident) => $result:ident ) => {
        impl<X,T1,T2,T3> Infer <($t1<T1>,$t2<T2>,$t3<T3>)> for X { type Result = $result<X>; }
    };
}

inference_rules! {
    EventMessage    => EventMessage
    BehaviorMessage => BehaviorMessage

    (EventMessage    , EventMessage   ) => EventMessage
    (BehaviorMessage , EventMessage   ) => EventMessage
    (EventMessage    , BehaviorMessage) => EventMessage
    (BehaviorMessage , BehaviorMessage) => EventMessage
}



// ============================
// === ContainsEventMessage ===
// ============================

pub trait ContainsEventMessage {
    type Result : Message;
}

pub type SelectEventMessage<T> = <T as ContainsEventMessage>::Result;

impl<T1> ContainsEventMessage for EventMessage<T1>
    where EventMessage<T1> : Message {
    type Result = EventMessage<T1>;
}

impl<T1,T2> ContainsEventMessage for (EventMessage<T1>,BehaviorMessage<T2>)
    where EventMessage<T1> : Message {
    type Result = EventMessage<T1>;
}

impl<T1,T2> ContainsEventMessage for (BehaviorMessage<T1>,EventMessage<T2>)
    where EventMessage<T2> : Message {
    type Result = EventMessage<T2>;
}

impl<T1,T2,T3> ContainsEventMessage for (EventMessage<T1>,BehaviorMessage<T2>,BehaviorMessage<T3>)
    where EventMessage<T1> : Message {
    type Result = EventMessage<T1>;
}

impl<T1,T2,T3> ContainsEventMessage for (BehaviorMessage<T1>,EventMessage<T2>,BehaviorMessage<T3>)
    where EventMessage<T2> : Message {
    type Result = EventMessage<T2>;
}

impl<T1,T2,T3> ContainsEventMessage for (BehaviorMessage<T1>,BehaviorMessage<T2>,EventMessage<T3>)
    where EventMessage<T3> : Message {
    type Result = EventMessage<T3>;
}



// =================================================================================================
// === FRP Nodes ===================================================================================
// =================================================================================================

// ==============
// === Source ===
// ==============

// === Storage ===

/// Internal source storage accessor.
pub type SourceStorage<T> = <T as KnownSourceStorage>::SourceStorage;

/// Internal source storage type.
pub trait KnownSourceStorage {
    /// The result type.
    type SourceStorage : Default;
}

impl<T>         KnownSourceStorage for EventMessage   <T> {type SourceStorage = ();}
impl<T:Default> KnownSourceStorage for BehaviorMessage<T> {type SourceStorage = BehaviorMessage<T>;}


// === Definition ===

/// Source is a begin point in the FRP network. It is able to emit events or initialize behaviors.
type Source<Out> = NodeWrapper<SourceShape<Out>>;

/// Internal definition of the source FRP node.
#[derive(Derivative)]
#[derivative(Default (bound="SourceStorage<Out>:Default"))]
#[derivative(Debug   (bound="SourceStorage<Out>:Debug"))]
pub struct SourceShape<Out:KnownSourceStorage> {
    storage: SourceStorage<Out>
}

impl<Out> KnownOutput for SourceShape<Out>
    where Out : KnownSourceStorage + Message {
    type Output = Out;
}

impl<Out> Source<Out>
    where Out : KnownSourceStorage + Message {
    /// Constructor.
    pub fn new_named<Label:Into<CowString>>(label:Label) -> Self {
        Self::construct(label,default())
    }
}

impl<Out> HasCurrentValue for Source<BehaviorMessage<Out>>
where Out : MessageValue {
    fn current_value(&self) -> Out {
        self.rc.borrow().shape.storage.value()
    }
}

impl<Out:KnownSourceStorage> HasInputs for SourceShape<Out> {
    fn inputs(&self) -> Vec<AnyNode> {
        default()
    }
}



macro_rules! define_node {
    (
        $(#$meta:tt)*
        pub struct $name:ident $shape_name:ident [$($poly_input:ident)*]
            { $( $field:ident : $field_type:ty ),* }
    ) => {
        $(#$meta)*
        pub type $name<$($poly_input,)* Out> = NodeWrapper<$shape_name<$($poly_input,)* Out>>;

        $(#$meta)*
        #[derive(Debug)]
        #[allow(non_camel_case_types)]
        pub struct $shape_name<$($poly_input:Message,)* Out:Message> {
            $( $poly_input : Node<$poly_input> ),* ,
            $( $field      : $field_type ),*
        }

        impl<$($poly_input:Message,)* Out:Message>
        KnownOutput for $shape_name<$($poly_input,)* Out> {
            type Output = Out;
        }

        impl<$($poly_input:Message,)* Out:Message>
        KnownEventInput for $shape_name<$($poly_input,)* Out>
        where ($($poly_input),*) : ContainsEventMessage,
              SelectEventMessage<($($poly_input),*)> : Message {
            type EventInput = SelectEventMessage<($($poly_input),*)>;
        }
    }
}



// =============
// === Merge ===
// =============

pub type Merge<T> = NodeWrapper<MergeShape<T>>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct MergeShape<T:Message> {
    source1 : Node<T>,
    source2 : Node<T>,
}

impl<T:Message> KnownOutput     for MergeShape<T> { type Output     = T; }
impl<T:Message> KnownEventInput for MergeShape<T> { type EventInput = T; }


// === Constructor ===

impl<T:Message> Merge<T>
    where Node<T> : AddTarget<Self> {
    fn new_named<Label,Source1,Source2>(label:Label, source1:Source1, source2:Source2) -> Self
        where Label   : Into<CowString>,
              Source1 : Into<Node<T>>,
              Source2 : Into<Node<T>> {
        let source1     = source1.into();
        let source2     = source2.into();
        let source1_ref = source1.clone();
        let source2_ref = source2.clone();
        let this        = Self::construct(label,MergeShape{source1,source2});
        source1_ref.add_target(&this);
        source2_ref.add_target(&this);
        this
    }
}

impl<T:MessageValue> EventConsumer for Merge<EventMessage<T>> {
    fn on_event(&self, event:&Self::EventInput) {
        self.emit_event(event);
    }
}

impl<T:Message> HasInputs for MergeShape<T> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source1).into(), (&self.source2).into()]
    }
}



// ==============
// === Toggle ===
// ==============

pub type Toggle<T> = NodeWrapper<ToggleShape<T>>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct ToggleShape<T:Message> {
    source : Node<T>,
    status : Cell<bool>,
}

impl<T:Message> KnownOutput     for ToggleShape<T> { type Output     = EventMessage<bool>; }
impl<T:Message> KnownEventInput for ToggleShape<T> { type EventInput = T; }


// === Constructor ===

impl<T:MessageValue> Toggle<EventMessage<T>> {
    fn new_named<Label,Source> (label:Label, source:Source) -> Self
        where Label  : Into<CowString>,
              Source : Into<Event<T>> {
        let status     = default();
        let source     = source.into();
        let source_ref = source.clone();
        let this       = Self::construct(label,ToggleShape{source,status});
        source_ref.add_target(&this);
        this
    }
}

impl<T:MessageValue> EventConsumer for Toggle<EventMessage<T>> {
    fn on_event(&self, _:&Self::EventInput) {
        let val = !self.rc.borrow().shape.status.get();
        self.rc.borrow().shape.status.set(val);
        self.emit_event(&EventMessage(val));
    }
}

impl<T:Message> HasInputs for ToggleShape<T> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source).into()]
    }
}



// ============
// === Hold ===
// ============

pub type Hold<T> = NodeWrapper<HoldShape<T>>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct HoldShape<T:Message> {
    source   : Node<T>,
    last_val : RefCell<Value<T>>,
}

impl<T:MessageValue> KnownOutput for HoldShape<EventMessage<T>> {
    type Output = BehaviorMessage<T>;
}
impl<T:Message> KnownEventInput for HoldShape<T> { type EventInput = T; }


// === Constructor ===

impl<T:MessageValue> Hold<EventMessage<T>>
    where Node<EventMessage<T>> : AddTarget<Self> {
    fn new_named<Label,Source>(label:Label, source:Source) -> Self
        where Label  : Into<CowString>,
              Source : Into<Node<EventMessage<T>>> {
        let last_val   = default();
        let source     = source.into();
        let source_ref = source.clone();
        let this       = Self::construct(label,HoldShape{source,last_val});
        source_ref.add_target(&this);
        this
    }
}

impl<T:MessageValue> EventConsumer for Hold<EventMessage<T>> {
    fn on_event(&self, event:&Self::EventInput) {
        *self.rc.borrow().shape.last_val.borrow_mut() = event.value().clone();
    }
}

impl<T> HasCurrentValue for Hold<EventMessage<T>>
where T : MessageValue {
    fn current_value(&self) -> T {
        self.rc.borrow().shape.last_val.borrow().clone()
    }
}

impl<T:Message> HasInputs for HoldShape<T> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source).into()]
    }
}



// =================
// === Recursive ===
// =================

pub type Recursive<T> = NodeWrapper<RecursiveShape2<T>>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct RecursiveShape2<T:Message> {
    source : RefCell<Option<Node<T>>>,
}

impl<T:Message> KnownOutput for RecursiveShape2<T> {
    type Output = T;
}

impl<T:Message> KnownEventInput for RecursiveShape2<T> {
    type EventInput = T;
}


// === Constructor ===

impl<T:Message> Recursive<T> {
    pub fn new_named<Label>(label:Label) -> Self
        where Label : Into<CowString> {
        let source = default();
        Self::construct(label,RecursiveShape2{source})
    }

    pub fn initialize<S>(&self, t:S)
        where S       : Into<Node<T>>,
              Node<T> : AddTarget<Self> {
        let node = t.into();
        node.add_target(self);
        self.set_display_id(node.display_id());
        *self.rc.borrow().shape.source.borrow_mut() = Some(node);
    }
}

impl<T:Message> EventConsumer for Recursive<T> {
    fn on_event(&self, event:&T) {
        self.emit_event(event);
    }
}

impl<T:MessageValue> HasCurrentValue for Recursive<BehaviorMessage<T>> {
    fn current_value(&self) -> T {
        self.rc.borrow().shape.source.borrow().as_ref().unwrap().current_value()
    }
}

impl<T:Message> HasInputs for RecursiveShape2<T> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![self.source.borrow().as_ref().unwrap().clone_ref().into()]
    }
}




// ==============
// === Sample ===
// ==============

pub type Sample<In1,In2> = NodeWrapper<SampleShape<In1,In2>>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct SampleShape<In1:Message,In2:Message> {
    source1 : Node<In1>,
    source2 : Node<In2>,
}

impl<In1,In2> KnownOutput for SampleShape<EventMessage<In1>,BehaviorMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type Output = EventMessage<In2>;
}

impl<In1,In2> KnownOutput for SampleShape<BehaviorMessage<In1>,EventMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type Output = EventMessage<In1>;
}

impl<In1,In2> KnownEventInput for SampleShape<EventMessage<In1>,BehaviorMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type EventInput = EventMessage<In1>;
}

impl<In1,In2> KnownEventInput for SampleShape<BehaviorMessage<In1>,EventMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type EventInput = EventMessage<In2>;
}


// === Constructor ===

impl<In1:Message, In2:Message> Sample<In1,In2>
    where Node<In1>            : AddTarget<Self>,
          Node<In2>            : AddTarget<Self>,
          SampleShape<In1,In2> : KnownOutput {
    fn new_named<Label,Source1,Source2> (label:Label, source1:Source1, source2:Source2) -> Self
        where Label   : Into<CowString>,
              Source1 : Into<Node<In1>>,
              Source2 : Into<Node<In2>> {
        let source1     = source1.into();
        let source2     = source2.into();
        let source1_ref = source1.clone();
        let source2_ref = source2.clone();
        let this        = Self::construct(label,SampleShape{source1,source2});
        source1_ref.add_target(&this);
        source2_ref.add_target(&this);
        this
    }
}

impl<In1,In2> EventConsumer for Sample<BehaviorMessage<In1>,EventMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    fn on_event(&self, _:&Self::EventInput) {
        let value = self.rc.borrow().shape.source1.current_value();
        self.emit_event(&EventMessage(value));
    }
}

impl<In1,In2> EventConsumer for Sample<EventMessage<In1>,BehaviorMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    fn on_event(&self, _:&Self::EventInput) {
        let value = self.rc.borrow().shape.source2.current_value();
        self.emit_event(&EventMessage(value));
    }
}

impl<In1:Message, In2:Message> HasInputs for SampleShape<In1,In2> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source1).into(), (&self.source2).into()]
    }
}



// ============
// === Gate ===
// ============

pub type Gate<In1,In2> = NodeWrapper<GateShape<In1,In2>>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct GateShape<In1:Message,In2:Message> {
    source1 : Node<In1>,
    source2 : Node<In2>,
}

impl<In1,In2> KnownOutput for GateShape<EventMessage<In1>,BehaviorMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type Output = EventMessage<In1>;
}

impl<In1,In2> KnownOutput for GateShape<BehaviorMessage<In1>,EventMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type Output = EventMessage<In2>;
}

impl<In1,In2> KnownEventInput for GateShape<EventMessage<In1>,BehaviorMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type EventInput = EventMessage<In1>;
}

impl<In1,In2> KnownEventInput for GateShape<BehaviorMessage<In1>,EventMessage<In2>>
    where In1:MessageValue, In2:MessageValue {
    type EventInput = EventMessage<In2>;
}


// === Constructor ===

impl<In2:MessageValue> Gate<BehaviorMessage<bool>,EventMessage<In2>> {
    fn new_named<Label,Source1,Source2> (label:Label, source1:Source1, source2:Source2) -> Self
        where Label   : Into<CowString>,
              Source1 : Into<Behavior<bool>>,
              Source2 : Into<Event<In2>> {
        let source1     = source1.into();
        let source2     = source2.into();
        let source1_ref = source1.clone();
        let source2_ref = source2.clone();
        let this        = Self::construct(label,GateShape{source1,source2});
        source1_ref.add_target(&this);
        source2_ref.add_target(&this);
        this
    }
}

impl<In:MessageValue> EventConsumer for Gate<BehaviorMessage<bool>,EventMessage<In>> {
    fn on_event(&self, event:&Self::EventInput) {
        let check = self.rc.borrow().shape.source1.current_value();
        if check {
            self.emit_event(event);
        }
    }
}

impl<In1:Message, In2:Message> HasInputs for GateShape<In1,In2> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source1).into(), (&self.source2).into()]
    }
}



// ==============
// === Lambda ===
// ==============

define_node! {
    /// Transforms input data with the provided function. Lambda accepts a single input and outputs
    /// message of the same type as the input message.
    pub struct Lambda LambdaShape [source] {
        func : Lambda1Func<source,Out>
    }
}


// === LambdaFunc ===

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Lambda1Func<In1:Message,Out:Message> {
    #[derivative(Debug="ignore")]
    raw : Rc<dyn Fn(&Value<In1>) -> Out>
}

impl<In1,Out,Func> From<Func> for Lambda1Func<In1,Out>
    where In1  : Message,
          Out  : Message,
          Func : 'static + Fn(&Value<In1>) -> Value<Out> {
    fn from(func:Func) -> Self {
        let raw = Rc::new(move |a:&Value<In1>| { wrap(func(a)) });
        Self {raw}
    }
}


// === Constructor ===

/// Constructor abstraction. Used only to satisfy Rust type system.
pub trait LambdaNew<Source,Func> {
    /// Constructor.
    fn new_named<Label:Into<CowString>>(label:Label, source:Source,f:Func) -> Self;
}

impl<In,OutVal,Func,Source> LambdaNew<Source,Func> for Lambda<In,Inferred<In,OutVal>>
    where In       : Message,
          OutVal   : Infer<In>,
          Func     : 'static + Fn(&Value<In>) -> OutVal,
          Source   : Into<Node<In>>,
          Node<In> : AddTarget<Self>,
          Inferred<In,OutVal> : Message<Content=OutVal> {
    fn new_named<Label>(label:Label, source:Source, func:Func) -> Self
        where Label : Into<CowString> {
        let source     = source.into();
        let source_ref = source.clone();
        let func       = func.into();
        let this       = Self::construct(label,LambdaShape{source,func});
        source_ref.add_target(&this);
        this
    }
}

impl<In:MessageValue,Out:Message> EventConsumer for Lambda<EventMessage<In>,Out> {
    fn on_event(&self, input:&Self::EventInput) {
        let output = (self.rc.borrow().shape.func.raw)(unwrap(input));
        self.emit_event(&output);
    }
}


pub fn trace<T,Label,Source>(label:Label, source:Source) -> Lambda<T,T>
    where T        : Message,
          Label    : Str,
          Source   : Into<Node<T>>,
          Value<T> : MessageValue + Infer<T,Result=T>,
          Node<T>  : AddTarget<Lambda<T,T>> {
    let label = label.into();
    Lambda::new_named("trace",source, move |t| {
        println!("TRACE [{}]: {:?}", label, t);
        t.clone()
    })
}

impl<In1:Message, Out:Message> HasInputs for LambdaShape<In1,Out> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source).into()]
    }
}



// ===============
// === Lambda2 ===
// ===============

define_node! {
    /// Transforms input data with the provided function. `Lambda2` accepts two inputs. If at least
    /// one of the inputs was event, the output message will be event as well. In case both inputs
    /// were behavior, a new behavior will be produced.
    pub struct Lambda2 Lambda2Shape [source1 source2] {
        func : Lambda2Func<source1,source2,Out>
    }
}


// === LambdaFunc ===

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Lambda2Func<In1:Message,In2:Message,Out:Message> {
    #[derivative(Debug="ignore")]
    raw : Rc<dyn Fn(&Value<In1>,&Value<In2>) -> Out>
}

impl<In1,In2,Out,Func> From<Func> for Lambda2Func<In1,In2,Out>
    where In1  : Message,
          In2  : Message,
          Out  : Message,
          Func : 'static + Fn(&Value<In1>,&Value<In2>) -> Value<Out> {
    fn from(func:Func) -> Self {
        let raw = Rc::new(move |a:&Value<In1>,b:&Value<In2>| { wrap(func(a,b)) });
        Self {raw}
    }
}


// === Construction ===

/// Constructor abstraction. Used only to satisfy Rust type system.
pub trait Lambda2New<Source1,Source2,Function> {
    /// Constructor.
    fn new_named<Label>(label:Label, source:Source1, source2:Source2, f:Function) -> Self
        where Label : Into<CowString>;
}

impl<In1,In2,OutVal,Source1,Source2,Function>
Lambda2New<Source1,Source2,Function> for Lambda2<In1,In2,Inferred<(In1,In2),OutVal>>
    where In1       : Message,
          In2       : Message,
          OutVal    : Infer<(In1,In2)>,
          Source1   : Into<Node<In1>>,
          Source2   : Into<Node<In2>>,
          Function  : 'static + Fn(&Value<In1>,&Value<In2>) -> OutVal,
          Node<In1> : AddTarget<Self>,
          Node<In2> : AddTarget<Self>,
          Inferred<(In1,In2),OutVal> : Message<Content=OutVal> {
    fn new_named<Label>(label:Label, source1:Source1, source2:Source2, func:Function) -> Self
        where Label : Into<CowString> {
        let source1     = source1.into();
        let source2     = source2.into();
        let source1_ref = source1.clone();
        let source2_ref = source2.clone();
        let func        = func.into();
        let this        = Self::construct(label,Lambda2Shape{source1,source2,func});
        source1_ref.add_target(&this);
        source2_ref.add_target(&this);
        this
    }
}

impl<In1,In2,Out> EventConsumer for Lambda2<EventMessage<In1>,BehaviorMessage<In2>,Out>
    where In1:MessageValue, In2:MessageValue, Out:Message {
    fn on_event(&self, event:&Self::EventInput) {
        let value2 = self.rc.borrow().shape.source2.current_value();
        let output = (self.rc.borrow().shape.func.raw)(&event.value(),&value2);
        self.emit_event(&output);
    }
}

impl<In1,In2,Out> EventConsumer for Lambda2<BehaviorMessage<In1>,EventMessage<In2>,Out>
    where In1:MessageValue, In2:MessageValue, Out:Message {
    fn on_event(&self, event:&Self::EventInput) {
        let value1 = self.rc.borrow().shape.source1.current_value();
        let output = (self.rc.borrow().shape.func.raw)(&value1,&event.value());
        self.emit_event(&output);
    }
}

impl<In1:Message, In2:Message, Out:Message> HasInputs for Lambda2Shape<In1,In2,Out> {
    fn inputs(&self) -> Vec<AnyNode> {
        vec![(&self.source1).into(),(&self.source2).into()]
    }
}



// =================================================================================================
// === Examples ====================================================================================
// =================================================================================================

macro_rules! frp {
    ( $( $var:ident = $node:ident $(<$ty:ty>)*   ($($args:tt)*); )* ) => {$(
        let $var = $node $(::<$ty>)* :: new_named(stringify!{$var}, $($args)* );
    )*}
}


pub struct Dynamic<Out:MessageValue> {
    pub event    : Event    <Out>,
    pub behavior : Behavior <Out>,
}

impl<Out:MessageValue> Dynamic<Out> {
    pub fn new<E,B>(event:E, behavior:B) -> Self
        where E:Into<Event<Out>>, B:Into<Behavior<Out>> {
        let event    = event.into();
        let behavior = behavior.into();
        Self {event,behavior}
    }

    pub fn merge<Label>(&self, label:Label, that:&Dynamic<Out>) -> Self
        where Label:Into<CowString> {
        (&Merge::new_named(label,self,that)).into()
    }

    pub fn toggle<Label>(&self, label:Label) -> Dynamic<bool>
        where Label:Into<CowString> {
        (&Toggle::new_named(label,self)).into()
    }

    pub fn gate<Label>(&self, label:Label, that:&Dynamic<bool>) -> Self
        where Label:Into<CowString> {
        (&Gate::new_named(label,that,self)).into()
    }

    pub fn sample<Label,T>(&self, label:Label, that:&Dynamic<T>) -> Self
        where Label : Into<CowString>,
              T     : MessageValue {
        (&Sample::new_named(label,&self.behavior,that)).into()
    }

    pub fn map<Label,F,R>(&self, label:Label, f:F) -> Dynamic<R>
        where Label : Into<CowString>,
              R     : MessageValue,
              F     : 'static + Fn(&Out) -> R {
        (&Lambda::new_named(label,&self.event,f)).into()
    }

    pub fn map2<Label,T,F,R>(&self, label:Label, that:&Dynamic<T>, f:F) -> Dynamic<R>
        where Label : Into<CowString>,
              T     : MessageValue,
              R     : MessageValue,
              F     : 'static + Fn(&Out,&T) -> R {
        (&Lambda2::new_named(label,&self.event,that,f)).into()
    }

    pub fn constant<Label,T>(&self, label:Label, value:T) -> Dynamic<T>
        where Label:Into<CowString>, T:MessageValue {
        self.map(label,move |_| value.clone())
    }
}

impl<Out:MessageValue> Dynamic<Out> {
    pub fn source<Label>(label:Label) -> Self
        where Label : Into<CowString> {
        let event = Source::<EventMessage<Out>>::new_named(label);
        (&event).into()
    }
}


impl<Out:MessageValue, T:Into<Event<Out>>> From<T> for Dynamic<Out> {
    fn from(t:T) -> Self {
        let event    = t.into();
        let behavior = Hold :: new_named(event.label(),&event);
        behavior.set_display_id(event.display_id());
        let event    = (&event).into();
        let behavior = (&behavior).into();
        Dynamic {event,behavior}
    }
}

impl<Out:MessageValue> From<&Dynamic<Out>> for Event<Out> {
    fn from(t:&Dynamic<Out>) -> Self {
        t.event.clone_ref()
    }
}

impl<Out:MessageValue> From<&Dynamic<Out>> for Behavior<Out> {
    fn from(t:&Dynamic<Out>) -> Self {
        t.behavior.clone_ref()
    }
}


//use std::concat;


//#[allow(missing_docs)]
//mod tests {
//    use super::*;
//
//    use crate::system::web;
//    use crate::control::io::mouse2;
//    use crate::control::io::mouse2::MouseManager;
//
//
//    // ================
//    // === Position ===
//    // ================
//
//    #[derive(Clone,Copy,Debug,Default)]
//    pub struct Position {
//        x:i32,
//        y:i32,
//    }
//
//    impl Position {
//        pub fn new(x:i32, y:i32) -> Self {
//            Self {x,y}
//        }
//    }
//
//    impl std::ops::Sub<&Position> for &Position {
//        type Output = Position;
//        fn sub(self, rhs: &Position) -> Self::Output {
//            let x = self.x - rhs.x;
//            let y = self.y - rhs.y;
//            Position {x,y}
//        }
//    }
//
//
//    macro_rules! frp_def {
//        ($var:ident = $fn:ident $(.$fn2:ident)* $(::<$ty:ty>)? ($($args:tt)*)) => {
//            let $var = Dynamic $(::<$ty>)? :: $fn $(.$fn2)*
//            ( concat! {stringify!{$var}}, $($args)* );
//        };
//
//        ($scope:ident . $var:ident = $fn:ident $(::<$ty:ty>)? ($($args:tt)*)) => {
//            let $var = Dynamic $(::<$ty>)? :: $fn
//            ( concat! {stringify!{$scope},".",stringify!{$var}}, $($args)* );
//        };
//
//        ($scope:ident . $var:ident = $fn1:ident . $fn2:ident $(.$fn3:ident)* $(::<$ty:ty>)? ($($args:tt)*)) => {
//            let $var = $fn1 . $fn2 $(.$fn3)* $(::<$ty>)?
//            ( concat! {stringify!{$scope},".",stringify!{$var}}, $($args)* );
//        };
//    }
//
//    // ============
//    // === Test ===
//    // ============
//
//    pub struct Mouse {
//        pub up       : Dynamic<()>,
//        pub down     : Dynamic<()>,
//        pub is_down  : Dynamic<bool>,
//        pub position : Dynamic<Position>,
//    }
//
//    impl Mouse {
//        pub fn new() -> Self {
//            frp_def! { mouse.up        = source() }
//            frp_def! { mouse.down      = source() }
//            frp_def! { mouse.position  = source() }
//            frp_def! { mouse.down_bool = down.constant(true) }
//            frp_def! { mouse.up_bool   = up.constant(false) }
//            frp_def! { mouse.is_down   = down_bool.merge(&up_bool) }
//            Self {up,down,is_down,position}
//        }
//    }
//
//    #[allow(unused_variables)]
//    pub fn test (callback: Box<dyn Fn(f32,f32)>) -> MouseManager {
//
//        let document        = web::document().unwrap();
//        let mouse_manager   = MouseManager::new(&document);
//
//
//
//        println!("\n\n\n--- FRP ---\n");
//
//
//        let mouse = Mouse::new();
//
//        let mouse_down_position    = mouse.position.sample("mouse_down_position",&mouse.down);
//        let mouse_position_if_down = mouse.position.gate("mouse_position_if_down",&mouse.is_down);
//
//        let final_position_ref_i  = Recursive::<EventMessage<Position>>::new_named("final_position_ref");
//        let final_position_ref    = Dynamic::from(&final_position_ref_i);
//
//        let pos_diff_on_down   = mouse_down_position.map2("pos_diff_on_down", &final_position_ref, |m,f| {m - f});
//        let final_position  = mouse_position_if_down.map2("final_position", &pos_diff_on_down, |m,f| {m - f});
//        let debug              = final_position.sample("debug", &mouse.position);
//
//
//
//        final_position_ref_i.initialize(&final_position);
//
//        final_position_ref.event.set_display_id(final_position.event.display_id());
//        final_position_ref.behavior.set_display_id(final_position.event.display_id());
//
//
//
//        trace("X" , &debug.event);
//
//
//        final_position.map("foo",move|p| {callback(p.x as f32,-p.y as f32)});
//
//        final_position.behavior.display_graphviz();
//
//        let target = mouse.position.event.clone_ref();
//        let handle = mouse_manager.on_move.add(move |event:&mouse2::event::OnMove| {
//            target.emit(&EventMessage(Position::new(event.client_x(),event.client_y())));
//        });
//        handle.forget();
//
//        let target = mouse.down.event.clone_ref();
//        let handle = mouse_manager.on_down.add(move |event:&mouse2::event::OnDown| {
//            target.emit(&EventMessage(()));
//        });
//        handle.forget();
//
//        let target = mouse.up.event.clone_ref();
//        let handle = mouse_manager.on_up.add(move |event:&mouse2::event::OnUp| {
//            target.emit(&EventMessage(()));
//        });
//        handle.forget();
//
//        mouse_manager
//
//    }
//}
//pub use tests::*;
