//! Entity Component System Library (ECS)
//!
//! For info about why an ECS may be beneficial, see some of these articles:
//!
//! - http://gameprogrammingpatterns.com/component.html
//! - http://t-machine.org/index.php/2007/09/03/entity-systems-are-the-future-of-mmog-development-part-1/
//! - http://www.gamedev.net/page/resources/_/technical/game-programming/understanding-component-entity-systems-r3013
//! - http://cowboyprogramming.com/2007/01/05/evolve-your-heirachy/
//!
//! There is a large variety of ways an ECS may work. This particular one is similar to
//! [Artemis](http://gamadu.com/artemis/).
//! Although this isn't a port to Rust, most functionality should be similar, and the
//! tutorials/manual there should be able to make up for the current lack of documentation [FIXME]
//!
//! Here's the basic structure:
//!
//! - An `Entity` is just an identifier. It contains no data or logic whatsoever.
//! - A `Component` is a piece of data (eg: Position, Velocity, Colour). While containing logic can
//! sometimes be useful, it's best practice to avoid it wherever possible.
//! - A `System` runs all the logic. Most of the time, it filters out entities based on their
//! components, and only runs it's logic on the entities it's interested in. These filters are
//! called `Aspect`s. Some systems ignore entities, and just apply logic to the world itself.
//! - An `Aspect` is a simple helper to filter entities based on their components.
//! - A `Manager` is simply an object that observes when an entity is
//! added/activated/deactivated/removed. They are used to help 'manage' the entities, rather than
//! define data or logic.
//! - The `World` organises all the above items together to make sure everything runs as it should.

#![crate_name = "ecs"]
#![crate_type = "lib"]

#![feature(collections)]

extern crate bincode;
extern crate rustc_serialize;

pub use aspect::Aspect;
pub use component::{Component, ComponentList};
pub use component::{EntityBuilder, EntityModifier};
pub use entity::{Entity, IndexedEntity, EntityIter};
pub use system::{System, Process};
pub use world::{ComponentManager, ServiceManager, SystemManager, DataHelper, World};

use std::ops::{Deref};

pub mod aspect;
pub mod component;
pub mod entity;
pub mod system;
pub mod world;

pub struct BuildData<'a, T: ComponentManager>(&'a IndexedEntity<T>);
pub struct ModifyData<'a, T: ComponentManager>(&'a IndexedEntity<T>);
pub struct EntityData<'a, T: ComponentManager>(&'a IndexedEntity<T>);
impl<'a, T: ComponentManager> Deref for EntityData<'a, T>
{
    type Target = IndexedEntity<T>;
    fn deref(&self) -> &IndexedEntity<T>
    {
        &self.0
    }
}

impl<'a, T: ComponentManager> Copy for BuildData<'a, T> {}
impl<'a, T: ComponentManager> Copy for ModifyData<'a, T> {}
impl<'a, T: ComponentManager> Copy for EntityData<'a, T> {}

impl<'a, T: ComponentManager> Clone for BuildData<'a, T> {fn clone(&self) -> BuildData<'a, T> {*self}}
impl<'a, T: ComponentManager> Clone for ModifyData<'a, T> {fn clone(&self) -> ModifyData<'a, T> {*self}}
impl<'a, T: ComponentManager> Clone for EntityData<'a, T> {fn clone(&self) -> EntityData<'a, T> {*self}}

#[doc(hidden)]
pub unsafe trait EditData<T: ComponentManager> { fn entity(&self) -> &IndexedEntity<T>; }
unsafe impl<'a, T: ComponentManager> EditData<T> for ModifyData<'a, T> { fn entity(&self) -> &IndexedEntity<T> { &self.0 } }
unsafe impl<'a, T: ComponentManager> EditData<T> for EntityData<'a, T> { fn entity(&self) -> &IndexedEntity<T> { &self.0 } }

#[macro_use]
mod macros
{
    #[macro_export]
    macro_rules! process {
        {
            $world:expr, $system:ident
        } => {
            $crate::Process::process(&mut $world.systems.$system, &mut $world.data)
        };
    }

    #[macro_export]
    macro_rules! components {
        {
            $Name:ident;
        } => {
            #[derive(RustcEncodable,RustcDecodable)]
            pub struct $Name;

            unsafe impl $crate::ComponentManager for $Name
            {
                unsafe fn new() -> $Name
                {
                    $Name
                }

                unsafe fn remove_all(&mut self, _: &$crate::IndexedEntity<$Name>)
                {

                }
            }
        };
        {
            $Name:ident {
                $(#[$kind:ident] $field_name:ident : $field_ty:ty),+
            }
        } => {
            #[derive(RustcEncodable,RustcDecodable)]
            pub struct $Name {
                $(
                    pub $field_name : $crate::ComponentList<$Name, $field_ty>,
                )+
            }

            unsafe impl $crate::ComponentManager for $Name
            {
                unsafe fn new() -> $Name
                {
                    $Name {
                        $(
                            $field_name : $crate::ComponentList::$kind(),
                        )+
                    }
                }

                unsafe fn remove_all(&mut self, entity: &$crate::IndexedEntity<$Name>)
                {
                    $(
                        self.$field_name.clear(entity);
                    )+
                }
            }
        };
        {
            $Name:ident {
                $(#[$kind:ident] $field_name:ident : $field_ty:ty),+,
            }
        } => {
            components! { $Name { $(#[$kind] $field_name : $field_ty),+ } }
        };
    }

    #[macro_export]
    macro_rules! services {
        {
            $Name:ident {
                $($field_name:ident : $field_ty:ty = $field_init:expr),+
            }
        } => {
            #[derive(RustcEncodable,RustcDecodable)]
            pub struct $Name {
                $(
                    pub $field_name : $field_ty,
                )+
            }

            impl $crate::ServiceManager for $Name
            {
                fn new() -> $Name
                {
                    $Name {
                        $(
                            $field_name : $field_init,
                        )+
                    }
                }
            }
        };
        {
            $Name:ident {
                $($field_name:ident : $field_ty:ty = $field_init:expr),+,
            }
        } => {
            services! { $Name { $($field_name : $field_ty = $field_init),+ } }
        }
    }

    #[macro_export]
    macro_rules! systems {
        {
            $Name:ident<$components:ty, $services:ty>;
        } => {
            pub struct $Name;

            unsafe impl $crate::SystemManager for $Name
            {
                type Components = $components;
                type Services = $services;
                #[allow(unused_unsafe)] // The aspect macro is probably going to be used here and it also expands to an unsafe block.
                unsafe fn new() -> $Name
                {
                    $Name
                }

                unsafe fn activated(&mut self, _: $crate::EntityData<$components>, _: &$components)
                {

                }

                unsafe fn reactivated(&mut self, _: $crate::EntityData<$components>, _: &$components)
                {

                }

                unsafe fn deactivated(&mut self, _: $crate::EntityData<$components>, _: &$components)
                {

                }

                unsafe fn update(&mut self, _: &mut $crate::DataHelper<$components, $services>)
                {

                }
            }
        };
        {
            $Name:ident<$components:ty, $services:ty> {
                $($field_name:ident : $field_ty:ty = $field_init:expr),+
            }
        } => {
            pub struct $Name {
                $(
                    pub $field_name : $field_ty,
                )+
            }

            unsafe impl $crate::SystemManager for $Name
            {
                type Components = $components;
                type Services = $services;
                #[allow(unused_unsafe)] // The aspect macro is probably going to be used here and it also expands to an unsafe block.
                unsafe fn new() -> $Name
                {
                    $Name {
                        $(
                            $field_name : $field_init,
                        )+
                    }
                }

                unsafe fn activated(&mut self, en: $crate::EntityData<$components>, co: &$components)
                {
                    $(
                        self.$field_name.activated(&en, co);
                    )+
                }

                unsafe fn reactivated(&mut self, en: $crate::EntityData<$components>, co: &$components)
                {
                    $(
                        self.$field_name.reactivated(&en, co);
                    )+
                }

                unsafe fn deactivated(&mut self, en: $crate::EntityData<$components>, co: &$components)
                {
                    $(
                        self.$field_name.deactivated(&en, co);
                    )+
                }

                unsafe fn update(&mut self, co: &mut $crate::DataHelper<$components, $services>)
                {
                    $(
                        if self.$field_name.is_active() {
                            $crate::Process::process(&mut self.$field_name, co);
                        }
                    )+
                }
            }
        };
        {
            $Name:ident<$components:ty, $services:ty> {
                $($field_name:ident : $field_ty:ty = $field_init:expr),+,
            }
        } => {
            systems! { $Name<$components, $services> { $($field_name : $field_ty = $field_init),+ } }
        }
    }

    #[macro_export]
    macro_rules! aspect {
        {
            <$components:ty>
            all: [$($all_field:ident),*]
            none: [$($none_field:ident),*]
        } => {
            unsafe {
                $crate::Aspect::new(Box::new(|_en: &$crate::EntityData<$components>, _co: &$components| {
                    ($(_co.$all_field.has(_en) &&)* true) &&
                    !($(_co.$none_field.has(_en) ||)* false)
                }))
            }
        };
        {
            <$components:ty>
            all: [$($field:ident),*]
        } => {
            aspect!(
                <$components>
                all: [$($field),*]
                none: []
            )
        };
        {
            <$components:ty>
            none: [$($field:ident),*]
        } => {
            aspect!(
                <$components>
                all: []
                none: [$($field),*]
            )
        };
    }
}
