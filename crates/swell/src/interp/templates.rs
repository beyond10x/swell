//! The work of the instruction of tagged templates (`#sec-tagged-templates`), GetTemplateObject
//! (`#sec-gettemplateobject`), called from one arm of the run loop (`interp/dispatch.rs`), so
//! that the loop's frame holds none of its locals. Written by `story:tagged-templates`.
//!
//! The realm's [[TemplateMap]] is kept with the code (`docs/design/engine.md`, decision 4,
//! "Template objects"): the script record of the running frame's code holds an entry per
//! tagged template site of the code (`script.rs`, `ScriptRecord::templates`), empty until the
//! site's first evaluation makes the template object and its `raw` Array; every later
//! evaluation answers that object and allocates nothing. The Arrays are made in the current
//! realm, the realm of the frame, which is the record's own. Steps: the instruction's, and at
//! the first evaluation one per element defined in the two Arrays, besides the step per key of
//! SetIntegrityLevel. No code runs and no safe point comes between the first allocation and the
//! entry that holds the object, so the Arrays need no root.

use std::rc::Rc;

use super::dispatch::Flow;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::property::{Descriptor, Key};
use crate::ops::objects::IntegrityLevel;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// `TemplateObject(k)`: GetTemplateObject of site `k` of the running frame's code, steps 1
    /// to 3 and 16: the record's entry for the site, made at the site's first evaluation, pushed.
    pub(super) fn template_object(&mut self, site: u32) -> Completion<Flow> {
        let script = self.top_frame()?.script;
        let index = usize::try_from(site).unwrap_or(usize::MAX);
        let entry = match self.heap.scripts.get(script) {
            Ok(record) => record.templates.get(index).copied(),
            Err(_) => None,
        };
        let template = match entry {
            Some(Some(template)) => template,
            Some(None) => {
                let template = self.make_template_object(index)?;
                let slot = self
                    .heap
                    .scripts
                    .get_mut(script)
                    .ok()
                    .and_then(|record| record.templates.get_mut(index));
                match slot {
                    Some(slot) => *slot = Some(template),
                    None => return Err(self.fault()),
                }
                template
            }
            None => return Err(self.fault()),
        };
        self.push_value(Value::Object(template))?;
        Ok(Flow::Next)
    }

    /// GetTemplateObject, steps 4 to 15, for site `index` of the running frame's code: an Array
    /// of the cooked strings (`undefined` for a part with a NotEscapeSequence) whose `raw` is an
    /// Array of the raw strings, each element { [[Writable]]: false, [[Enumerable]]: true,
    /// [[Configurable]]: false }, `raw` { [[Writable]]: false, [[Enumerable]]: false,
    /// [[Configurable]]: false }, both frozen.
    fn make_template_object(&mut self, index: usize) -> Completion<ObjectRef> {
        let compiled = Rc::clone(&self.top_frame()?.compiled);
        let Some(site) = compiled.code.template_sites.get(index) else {
            return Err(self.fault());
        };
        // 7. Let count be the number of elements in the List cookedStrings.
        let count = site.raw.len();
        let Ok(length) = u32::try_from(count) else {
            return Err(self.fault());
        };
        if site.cooked.len() != count {
            return Err(self.fault());
        }
        // 9. Let template be ! ArrayCreate(count). 10. Let rawObj be ! ArrayCreate(count).
        let template = self.array_create(f64::from(length), None)?;
        let raw = self.array_create(f64::from(length), None)?;
        // 12. Repeat, while index < count.
        for (i, (cooked, &raw_string)) in site.cooked.iter().zip(&site.raw).enumerate() {
            let Ok(i) = u32::try_from(i) else {
                return Err(self.fault());
            };
            let key = Key::Index(i);
            // 12.b, c: the cooked value, a String or undefined.
            self.charge_steps(1)?;
            let cooked = match *cooked {
                Some(k) => Value::String(self.template_string(&compiled, k)?),
                None => Value::Undefined,
            };
            self.define_property_or_throw(
                template,
                key,
                Descriptor::data(cooked, false, true, false),
            )?;
            // 12.d, e: the raw value.
            self.charge_steps(1)?;
            let raw_value = Value::String(self.template_string(&compiled, raw_string)?);
            self.define_property_or_throw(
                raw,
                key,
                Descriptor::data(raw_value, false, true, false),
            )?;
        }
        // 13. Perform ! SetIntegrityLevel(rawObj, frozen).
        self.set_integrity_level(raw, IntegrityLevel::Frozen)?;
        // 14. Perform ! DefinePropertyOrThrow(template, "raw", ...).
        let key = self.name_key("raw")?;
        self.define_property_or_throw(
            template,
            key,
            Descriptor::data(Value::Object(raw), false, false, false),
        )?;
        // 15. Perform ! SetIntegrityLevel(template, frozen).
        self.set_integrity_level(template, IntegrityLevel::Frozen)?;
        Ok(template)
    }

    /// The atom of String constant `k` of `compiled`.
    fn template_string(
        &mut self,
        compiled: &crate::script::Compiled,
        k: u32,
    ) -> Completion<crate::value::StringRef> {
        match compiled
            .strings
            .get(usize::try_from(k).unwrap_or(usize::MAX))
        {
            Some(&string) => Ok(string),
            None => Err(self.fault()),
        }
    }
}
