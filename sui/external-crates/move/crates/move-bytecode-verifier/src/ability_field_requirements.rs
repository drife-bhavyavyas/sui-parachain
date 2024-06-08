// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module implements a checker for verifying that all of the struct's fields satisfy the
//! abilities required by the struct's abilities
use move_binary_format::{
    errors::{verification_error, Location, PartialVMResult, VMResult},
    file_format::{AbilitySet, CompiledModule, StructFieldInformation, TableIndex},
    IndexKind,
};
use move_core_types::vm_status::StatusCode;

pub fn verify_module(module: &CompiledModule) -> VMResult<()> {
    verify_module_impl(module).map_err(|e| e.finish(Location::Module(module.self_id())))
}

fn verify_module_impl(module: &CompiledModule) -> PartialVMResult<()> {
    for (idx, struct_def) in module.struct_defs().iter().enumerate() {
        let sh = module.datatype_handle_at(struct_def.struct_handle);
        let fields = match &struct_def.field_information {
            StructFieldInformation::Native => continue,
            StructFieldInformation::Declared(fields) => fields,
        };
        let required_abilities = sh
            .abilities
            .into_iter()
            .map(|a| a.requires())
            .fold(AbilitySet::EMPTY, |acc, required| acc | required);
        // Assume type parameters have all abilities, as the struct's abilities will be dependent on
        // them
        let type_parameter_abilities = sh
            .type_parameters
            .iter()
            .map(|_| AbilitySet::ALL)
            .collect::<Vec<_>>();
        for field in fields {
            let field_abilities =
                module.abilities(&field.signature.0, &type_parameter_abilities)?;
            if !required_abilities.is_subset(field_abilities) {
                return Err(verification_error(
                    StatusCode::FIELD_MISSING_TYPE_ABILITY,
                    IndexKind::StructDefinition,
                    idx as TableIndex,
                ));
            }
        }
    }

    for (idx, enum_def) in module.enum_defs().iter().enumerate() {
        let sh = module.datatype_handle_at(enum_def.enum_handle);
        let required_abilities = sh
            .abilities
            .into_iter()
            .map(|a| a.requires())
            .fold(AbilitySet::EMPTY, |acc, required| acc | required);
        // Assume type parameters have all abilities, as the enum's abilities will be dependent on
        // them
        let type_parameter_abilities = sh
            .type_parameters
            .iter()
            .map(|_| AbilitySet::ALL)
            .collect::<Vec<_>>();
        for (i, variant) in enum_def.variants.iter().enumerate() {
            for (fi, field) in variant.fields.iter().enumerate() {
                let field_abilities =
                    module.abilities(&field.signature.0, &type_parameter_abilities)?;
                if !required_abilities.is_subset(field_abilities) {
                    return Err(verification_error(
                        StatusCode::FIELD_MISSING_TYPE_ABILITY,
                        IndexKind::EnumDefinition,
                        idx as TableIndex,
                    )
                    .at_index(IndexKind::VariantTag, i as TableIndex)
                    .at_index(IndexKind::FieldDefinition, fi as TableIndex));
                }
            }
        }
    }
    Ok(())
}