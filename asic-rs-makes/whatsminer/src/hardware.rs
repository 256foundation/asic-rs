use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::WhatsMinerModel;

impl From<WhatsMinerModel> for MinerHardware {
    fn from(value: WhatsMinerModel) -> Self {
        match value {
            WhatsMinerModel::M20PV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M20PV30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(148), Some(148), Some(148)]),
            },
            WhatsMinerModel::M20SPlusV30 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M20SV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M20SV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M20SV30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(140), Some(140), Some(140)]),
            },
            WhatsMinerModel::M20V10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M21SPlusV20 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M21SV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M21SV60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M21SV70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M21V10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(33), Some(33), Some(33)]),
            },
            WhatsMinerModel::M29V10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(50), Some(50), Some(50)]),
            },
            WhatsMinerModel::M30KV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M30LV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(144), Some(144), Some(144), Some(144)]),
            },
            WhatsMinerModel::M30SPlusPlusV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(255), Some(255), Some(255), Some(255)]),
            },
            WhatsMinerModel::M30SPlusPlusV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(255), Some(255), Some(255), Some(255)]),
            },
            WhatsMinerModel::M30SPlusPlusVE30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M30SPlusPlusVE40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M30SPlusPlusVE50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(235), Some(235), Some(235)]),
            },
            WhatsMinerModel::M30SPlusPlusVF40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M30SPlusPlusVG30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SPlusPlusVG40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M30SPlusPlusVG50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M30SPlusPlusVH10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M30SPlusPlusVH100 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M30SPlusPlusVH110 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30SPlusPlusVH20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M30SPlusPlusVH30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SPlusPlusVH40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SPlusPlusVH50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SPlusPlusVH60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M30SPlusPlusVH70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SPlusPlusVH80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SPlusPlusVH90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M30SPlusPlusVHA0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M30SPlusPlusVHB0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30SPlusPlusVI30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SPlusPlusVJ20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SPlusPlusVJ30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SPlusPlusVJ50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M30SPlusPlusVJ60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SPlusPlusVJ70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M30SPlusPlusVK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M30SPlusPlusVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SPlusPlusVK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30SPlusV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M30SPlusV100 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M30SPlusV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(255), Some(255), Some(255)]),
            },
            WhatsMinerModel::M30SPlusV30 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SPlusV40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(235), Some(235), Some(235)]),
            },
            WhatsMinerModel::M30SPlusV50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M30SPlusV60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M30SPlusV70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(235), Some(235), Some(235)]),
            },
            WhatsMinerModel::M30SPlusV80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M30SPlusV90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M30SPlusVA0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M30SPlusVE100 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SPlusVE30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(148), Some(148), Some(148)]),
            },
            WhatsMinerModel::M30SPlusVE40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M30SPlusVE50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M30SPlusVE60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M30SPlusVE70 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SPlusVE80 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SPlusVE90 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SPlusVF20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SPlusVF30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M30SPlusVG20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M30SPlusVG30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M30SPlusVG40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30SPlusVG50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SPlusVG60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M30SPlusVH10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(64), Some(64), Some(64)]),
            },
            WhatsMinerModel::M30SPlusVH20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M30SPlusVH30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SPlusVH40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SPlusVH50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(64), Some(64), Some(64)]),
            },
            WhatsMinerModel::M30SPlusVH60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M30SPlusVH70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SPlusVI30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M30SPlusVJ30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30SPlusVJ40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M30SV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(148), Some(148), Some(148)]),
            },
            WhatsMinerModel::M30SV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M30SV30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M30SV40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M30SV50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M30SV60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M30SV70 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SV80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M30SVE10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30SVE20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M30SVE30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M30SVE40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M30SVE50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M30SVE60 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SVE70 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SVF10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SVF20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SVF30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M30SVG10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M30SVG20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SVG30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M30SVG40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M30SVH10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(64), Some(64), Some(64)]),
            },
            WhatsMinerModel::M30SVH20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M30SVH30 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M30SVH40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(64), Some(64), Some(64)]),
            },
            WhatsMinerModel::M30SVH50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M30SVH60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SVI20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M30SVJ30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30V10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M30V20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31HV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(114), Some(114), Some(114)]),
            },
            WhatsMinerModel::M31HV40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(136), Some(136), Some(136), Some(136)]),
            },
            WhatsMinerModel::M31LV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(144), Some(144), Some(144), Some(144)]),
            },
            WhatsMinerModel::M31SPlusV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M31SPlusV100 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31SPlusV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31SPlusV30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M31SPlusV40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M31SPlusV50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(148), Some(148), Some(148)]),
            },
            WhatsMinerModel::M31SPlusV60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M31SPlusV80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M31SPlusV90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M31SPlusVA0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31SPlusVE10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M31SPlusVE20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M31SPlusVE30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M31SPlusVE40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31SPlusVE50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M31SPlusVE60 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M31SPlusVE80 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M31SPlusVF20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M31SPlusVF30 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M31SPlusVG20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M31SPlusVG30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M31SEV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M31SEV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M31SEV30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M31SV10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M31SV20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31SV30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M31SV40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M31SV50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M31SV60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M31SV70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M31SV80 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M31SV90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M31SVE10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M31SVE20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M31SVE30 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M31V10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            WhatsMinerModel::M31V20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M32V10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M32V20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M33SPlusPlusVG40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(174), Some(174), Some(174), Some(174)]),
            },
            WhatsMinerModel::M33SPlusPlusVH20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(112), Some(112), Some(112), Some(112)]),
            },
            WhatsMinerModel::M33SPlusPlusVH30 => Self {
                fans: Some(0),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M33SPlusVG20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(112), Some(112), Some(112), Some(112)]),
            },
            WhatsMinerModel::M33SPlusVG30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(162), Some(162), Some(162), Some(162)]),
            },
            WhatsMinerModel::M33SPlusVH20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(100), Some(100), Some(100), Some(100)]),
            },
            WhatsMinerModel::M33SPlusVH30 => Self {
                fans: Some(0),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M33SVG30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(116), Some(116), Some(116), Some(116)]),
            },
            WhatsMinerModel::M33V10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(33), Some(33), Some(33)]),
            },
            WhatsMinerModel::M33V20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(62), Some(62), Some(62)]),
            },
            WhatsMinerModel::M33V30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(66), Some(66), Some(66)]),
            },
            WhatsMinerModel::M34SPlusVE10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(116), Some(116), Some(116), Some(116)]),
            },
            WhatsMinerModel::M36SPlusPlusVH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(80), Some(80), Some(80), Some(80)]),
            },
            WhatsMinerModel::M36SPlusVG30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(108), Some(108), Some(108), Some(108)]),
            },
            WhatsMinerModel::M36SVE10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(114), Some(114), Some(114), Some(114)]),
            },
            WhatsMinerModel::M39V10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(50), Some(50), Some(50)]),
            },
            WhatsMinerModel::M39V20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(54), Some(54), Some(54)]),
            },
            WhatsMinerModel::M39V30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(68), Some(68), Some(68)]),
            },
            WhatsMinerModel::M50SPlusPlusVK10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50SPlusPlusVK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M50SPlusPlusVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M50SPlusPlusVK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M50SPlusPlusVK50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M50SPlusPlusVK60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SPlusPlusVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M50SPlusPlusVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M50SPlusPlusVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SPlusPlusVL40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50SPlusPlusVL50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M50SPlusPlusVL60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SPlusVH30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M50SPlusVH40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M50SPlusVJ30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M50SPlusVJ40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M50SPlusVJ60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M50SPlusVK10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SPlusVK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50SPlusVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M50SPlusVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M50SPlusVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M50SPlusVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M50SVH10 => Self {
                fans: Some(2),
                boards: Some(vec![None, None, None]),
            },
            WhatsMinerModel::M50SVH20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M50SVH30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M50SVH40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(148), Some(148), Some(148)]),
            },
            WhatsMinerModel::M50SVH50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M50SVJ10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SVJ20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50SVJ30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M50SVJ40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M50SVJ50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M50SVK10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M50SVK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50SVK50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M50SVK60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50SVK70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M50SVK80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M50SVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(74), Some(74), Some(74)]),
            },
            WhatsMinerModel::M50SVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M50SVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M50VE30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(255), Some(255), Some(255), Some(255)]),
            },
            WhatsMinerModel::M50VG30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(156), Some(156), Some(156)]),
            },
            WhatsMinerModel::M50VH10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M50VH20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50VH30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50VH40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(84), Some(84), Some(84)]),
            },
            WhatsMinerModel::M50VH50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M50VH60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(84), Some(84), Some(84)]),
            },
            WhatsMinerModel::M50VH70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(105), Some(105), Some(105)]),
            },
            WhatsMinerModel::M50VH80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50VH90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50VJ10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M50VJ20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50VJ30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M50VJ40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M50VJ60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M50VK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M50VK50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M51SPlusVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M52SPlusPlusVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(87), Some(87), Some(87), Some(87)]),
            },
            WhatsMinerModel::M52SVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(62), Some(62), Some(62), Some(62)]),
            },
            WhatsMinerModel::M53HVH10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(56), Some(56), Some(56), Some(56)]),
            },
            WhatsMinerModel::M53SPlusPlusVK10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(198), Some(198), Some(198), Some(198)]),
            },
            WhatsMinerModel::M53SPlusPlusVK20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(192), Some(192), Some(192), Some(192)]),
            },
            WhatsMinerModel::M53SPlusPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M53SPlusPlusVK50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(186), Some(186), Some(186), Some(186)]),
            },
            WhatsMinerModel::M53SPlusPlusVK70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(216), Some(216), Some(216), Some(216)]),
            },
            WhatsMinerModel::M53SPlusPlusVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(128), Some(128), Some(128), Some(128)]),
            },
            WhatsMinerModel::M53SPlusPlusVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(174), Some(174), Some(174), Some(174)]),
            },
            WhatsMinerModel::M53SPlusPlusVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(168), Some(168), Some(168), Some(168)]),
            },
            WhatsMinerModel::M53SPlusPlusVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(128), Some(128), Some(128), Some(128)]),
            },
            WhatsMinerModel::M53SPlusPlusVL80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(144), Some(144), Some(144), Some(144)]),
            },
            WhatsMinerModel::M53SPlusVJ30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M53SPlusVJ40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(248), Some(248), Some(248), Some(248)]),
            },
            WhatsMinerModel::M53SPlusVJ50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(264), Some(264), Some(264), Some(264)]),
            },
            WhatsMinerModel::M53SPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(168), Some(168), Some(168), Some(168)]),
            },
            WhatsMinerModel::M53SVH20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(198), Some(198), Some(198), Some(198)]),
            },
            WhatsMinerModel::M53SVH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(204), Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M53SVH40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M53SVJ30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(180), Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M53SVJ40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(192), Some(192), Some(192), Some(192)]),
            },
            WhatsMinerModel::M53SVJ50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M53SVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(128), Some(128), Some(128), Some(128)]),
            },
            WhatsMinerModel::M53VH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(128), Some(128), Some(128), Some(128)]),
            },
            WhatsMinerModel::M53VH40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(174), Some(174), Some(174), Some(174)]),
            },
            WhatsMinerModel::M53VH50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(162), Some(162), Some(162), Some(162)]),
            },
            WhatsMinerModel::M53VK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(100), Some(100), Some(100), Some(100)]),
            },
            WhatsMinerModel::M53VK60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(100), Some(100), Some(100), Some(100)]),
            },
            WhatsMinerModel::M54SPlusPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(96), Some(96), Some(96), Some(96)]),
            },
            WhatsMinerModel::M54SPlusPlusVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(68), Some(68), Some(68), Some(68)]),
            },
            WhatsMinerModel::M54SPlusPlusVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(90), Some(90), Some(90), Some(90)]),
            },
            WhatsMinerModel::M54SPlusVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(84), Some(84), Some(84), Some(84)]),
            },
            WhatsMinerModel::M54SVH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(120), Some(120), Some(120), Some(120)]),
            },
            WhatsMinerModel::M54SVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(102), Some(102), Some(102), Some(102)]),
            },
            WhatsMinerModel::M56SPlusPlusVK10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(160), Some(160), Some(160), Some(160)]),
            },
            WhatsMinerModel::M56SPlusPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(176), Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M56SPlusPlusVK40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(132), Some(132), Some(132), Some(132)]),
            },
            WhatsMinerModel::M56SPlusPlusVK50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(152), Some(152), Some(152), Some(152)]),
            },
            WhatsMinerModel::M56SPlusVJ30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(176), Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M56SPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(108), Some(108), Some(108), Some(108)]),
            },
            WhatsMinerModel::M56SPlusVK40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(114), Some(114), Some(114), Some(114)]),
            },
            WhatsMinerModel::M56SPlusVK50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(120), Some(120), Some(120), Some(120)]),
            },
            WhatsMinerModel::M56SVH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(152), Some(152), Some(152), Some(152)]),
            },
            WhatsMinerModel::M56SVJ30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(132), Some(132), Some(132), Some(132)]),
            },
            WhatsMinerModel::M56SVJ40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(152), Some(152), Some(152), Some(152)]),
            },
            WhatsMinerModel::M56VH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(108), Some(108), Some(108), Some(108)]),
            },
            WhatsMinerModel::M59VH30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(132), Some(132), Some(132), Some(132)]),
            },
            WhatsMinerModel::M60SPlusPlusVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M60SPlusPlusVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M60SPlusPlusVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M60SPlusPlusVL40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(235), Some(235), Some(235)]),
            },
            WhatsMinerModel::M60SPlusPlusVL50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M60SPlusPlusVL60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(196), Some(196), Some(196)]),
            },
            WhatsMinerModel::M60SPlusPlusVL70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(294), Some(294), Some(294)]),
            },
            WhatsMinerModel::M60SPlusPlusVL80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(196), Some(196), Some(196)]),
            },
            WhatsMinerModel::M60SPlusPlusVL90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M60SPlusPlusVLA0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M60SPlusPlusVLB0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M60SPlusPlusVM30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M60SPlusPlusVM40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M60SPlusPlusVM50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M60SPlusPlusVM60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M60SPlusPlusVM70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(141), Some(141), Some(141)]),
            },
            WhatsMinerModel::M60SPlusVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M60SPlusVK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M60SPlusVK50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M60SPlusVK60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(294), Some(294), Some(294)]),
            },
            WhatsMinerModel::M60SPlusVK70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(306), Some(306), Some(306)]),
            },
            WhatsMinerModel::M60SPlusVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(196), Some(196), Some(196)]),
            },
            WhatsMinerModel::M60SPlusVL100 => Self {
                fans: Some(2),
                boards: Some(vec![Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M60SPlusVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(147), Some(147), Some(147)]),
            },
            WhatsMinerModel::M60SPlusVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M60SPlusVL40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(188), Some(188), Some(188)]),
            },
            WhatsMinerModel::M60SPlusVL50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M60SPlusVL60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M60SPlusVL70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M60SPlusVL80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M60SPlusVL90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(184), Some(184), Some(184)]),
            },
            WhatsMinerModel::M60SPlusVLA0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M60SPlusVLB0 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M60SPlusVM20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M60SPlusVM30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M60SPlusVM40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(90), Some(90), Some(90)]),
            },
            WhatsMinerModel::M60SPlusVM50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(98), Some(98), Some(98)]),
            },
            WhatsMinerModel::M60SVK10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M60SVK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(235), Some(235), Some(235)]),
            },
            WhatsMinerModel::M60SVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M60SVK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M60SVK60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(188), Some(188), Some(188)]),
            },
            WhatsMinerModel::M60SVK70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(196), Some(196), Some(196)]),
            },
            WhatsMinerModel::M60SVK80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M60SVK90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(192), Some(192), Some(192)]),
            },
            WhatsMinerModel::M60SVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(147), Some(147), Some(147)]),
            },
            WhatsMinerModel::M60SVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M60SVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M60SVL40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M60SVL50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(188), Some(188), Some(188)]),
            },
            WhatsMinerModel::M60SVL60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(196), Some(196), Some(196)]),
            },
            WhatsMinerModel::M60SVL70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(141), Some(141), Some(141)]),
            },
            WhatsMinerModel::M60SVL80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M60SVL90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(141), Some(141), Some(141)]),
            },
            WhatsMinerModel::M60SVM20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            WhatsMinerModel::M60SVM40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(86), Some(86), Some(86)]),
            },
            WhatsMinerModel::M60VK10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M60VK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M60VK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(215), Some(215), Some(215)]),
            },
            WhatsMinerModel::M60VK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M60VK6A => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M60VL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(111), Some(111), Some(111)]),
            },
            WhatsMinerModel::M60VL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M60VL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(123), Some(123), Some(123)]),
            },
            WhatsMinerModel::M60VL40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(129), Some(129), Some(129)]),
            },
            WhatsMinerModel::M60VL50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M60VM40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(82), Some(82), Some(82)]),
            },
            WhatsMinerModel::M61SPlusVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M61SPlusVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M61SVK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M61SVK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(235), Some(235), Some(235)]),
            },
            WhatsMinerModel::M61SVL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(164), Some(164), Some(164)]),
            },
            WhatsMinerModel::M61SVL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M61SVL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M61SVL60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M61SVL70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(147), Some(147), Some(147)]),
            },
            WhatsMinerModel::M61SVL90 => Self {
                fans: Some(2),
                boards: Some(vec![Some(225), Some(225), Some(225)]),
            },
            WhatsMinerModel::M61SVM30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(117), Some(117), Some(117)]),
            },
            WhatsMinerModel::M61VK10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M61VK20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(184), Some(184), Some(184)]),
            },
            WhatsMinerModel::M61VK30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(188), Some(188), Some(188)]),
            },
            WhatsMinerModel::M61VK40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(192), Some(192), Some(192)]),
            },
            WhatsMinerModel::M61VK60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(188), Some(188), Some(188)]),
            },
            WhatsMinerModel::M61VK70 => Self {
                fans: Some(2),
                boards: Some(vec![Some(172), Some(172), Some(172)]),
            },
            WhatsMinerModel::M61VL10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M61VL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(141), Some(141), Some(141)]),
            },
            WhatsMinerModel::M61VL40 => Self {
                fans: Some(2),
                boards: Some(vec![Some(144), Some(144), Some(144)]),
            },
            WhatsMinerModel::M61VL50 => Self {
                fans: Some(2),
                boards: Some(vec![Some(147), Some(147), Some(147)]),
            },
            WhatsMinerModel::M61VL60 => Self {
                fans: Some(2),
                boards: Some(vec![Some(150), Some(150), Some(150)]),
            },
            WhatsMinerModel::M62SPlusPlusVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(132), Some(132), Some(132)]),
            },
            WhatsMinerModel::M62SPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(430), Some(430), Some(430)]),
            },
            WhatsMinerModel::M63SPlusPlusVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(360), Some(360), Some(360), Some(360)]),
            },
            WhatsMinerModel::M63SPlusPlusVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(380), Some(380), Some(380), Some(380)]),
            },
            WhatsMinerModel::M63SPlusPlusVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(304), Some(304), Some(304), Some(304)]),
            },
            WhatsMinerModel::M63SPlusPlusVL50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(340), Some(340), Some(340), Some(340)]),
            },
            WhatsMinerModel::M63SPlusPlusVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(380), Some(380), Some(380), Some(380)]),
            },
            WhatsMinerModel::M63SPlusPlusVL70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M63SPlusPlusVM10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(152), Some(152), Some(152), Some(152)]),
            },
            WhatsMinerModel::M63SPlusPlusVM20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(186), Some(186), Some(186), Some(186)]),
            },
            WhatsMinerModel::M63SPlusPlusVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(198), Some(198), Some(198), Some(198)]),
            },
            WhatsMinerModel::M63SPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(456), Some(456), Some(456), Some(456)]),
            },
            WhatsMinerModel::M63SPlusVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(304), Some(304), Some(304), Some(304)]),
            },
            WhatsMinerModel::M63SPlusVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(340), Some(340), Some(340), Some(340)]),
            },
            WhatsMinerModel::M63SPlusVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(370), Some(370), Some(370), Some(370)]),
            },
            WhatsMinerModel::M63SPlusVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M63SPlusVL50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(272), Some(272), Some(272), Some(272)]),
            },
            WhatsMinerModel::M63SPlusVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(304), Some(304), Some(304), Some(304)]),
            },
            WhatsMinerModel::M63SPlusVL70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M63SPlusVL80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(256), Some(256), Some(256), Some(256)]),
            },
            WhatsMinerModel::M63SPlusVL90 => Self {
                fans: Some(0),
                boards: Some(vec![Some(256), Some(256), Some(256), Some(256)]),
            },
            WhatsMinerModel::M63SPlusVLA0 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M63SPlusVLC0 => Self {
                fans: Some(0),
                boards: Some(vec![Some(222), Some(222), Some(222), Some(222)]),
            },
            WhatsMinerModel::M63SPlusVLD0 => Self {
                fans: Some(0),
                boards: Some(vec![Some(340), Some(340), Some(340), Some(340)]),
            },
            WhatsMinerModel::M63SPlusVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(136), Some(136), Some(136), Some(136)]),
            },
            WhatsMinerModel::M63SPlusVM40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(144), Some(144), Some(144), Some(144)]),
            },
            WhatsMinerModel::M63SVK10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(340), Some(340), Some(340), Some(340)]),
            },
            WhatsMinerModel::M63SVK20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(350), Some(350), Some(350), Some(350)]),
            },
            WhatsMinerModel::M63SVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(370), Some(370), Some(370), Some(370)]),
            },
            WhatsMinerModel::M63SVK40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M63SVK50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(300), Some(300), Some(300), Some(300)]),
            },
            WhatsMinerModel::M63SVK60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(350), Some(350), Some(350), Some(350)]),
            },
            WhatsMinerModel::M63SVK70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(340), Some(340), Some(340), Some(340)]),
            },
            WhatsMinerModel::M63SVK80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M63SVK90 => Self {
                fans: Some(0),
                boards: Some(vec![Some(304), Some(304), Some(304), Some(304)]),
            },
            WhatsMinerModel::M63SVKA0 => Self {
                fans: Some(0),
                boards: Some(vec![Some(272), Some(272), Some(272), Some(272)]),
            },
            WhatsMinerModel::M63SVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M63SVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(216), Some(216), Some(216), Some(216)]),
            },
            WhatsMinerModel::M63SVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(272), Some(272), Some(272), Some(272)]),
            },
            WhatsMinerModel::M63SVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(204), Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M63SVL50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M63SVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M63SVL70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M63SVL80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M63SVL90 => Self {
                fans: Some(0),
                boards: Some(vec![Some(128), Some(128), Some(128), Some(128)]),
            },
            WhatsMinerModel::M63SVLA0 => Self {
                fans: Some(0),
                boards: Some(vec![Some(256), Some(256), Some(256), Some(256)]),
            },
            WhatsMinerModel::M63SVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(132), Some(132), Some(132), Some(132)]),
            },
            WhatsMinerModel::M63VK10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(256), Some(256), Some(256), Some(256)]),
            },
            WhatsMinerModel::M63VK20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(264), Some(264), Some(264), Some(264)]),
            },
            WhatsMinerModel::M63VK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(272), Some(272), Some(272), Some(272)]),
            },
            WhatsMinerModel::M63VL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(174), Some(174), Some(174), Some(174)]),
            },
            WhatsMinerModel::M63VL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(204), Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M63VL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(216), Some(216), Some(216), Some(216)]),
            },
            WhatsMinerModel::M63VL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(180), Some(180), Some(180), Some(180)]),
            },
            WhatsMinerModel::M63VL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(216), Some(216), Some(216), Some(216)]),
            },
            WhatsMinerModel::M63VL70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(174), Some(174), Some(174), Some(174)]),
            },
            WhatsMinerModel::M64SPlusPlusVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(96), Some(96), Some(96), Some(96)]),
            },
            WhatsMinerModel::M64SVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(114), Some(114), Some(114), Some(114)]),
            },
            WhatsMinerModel::M64SVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(120), Some(120), Some(120), Some(120)]),
            },
            WhatsMinerModel::M64SVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(152), Some(152), Some(152), Some(152)]),
            },
            WhatsMinerModel::M64VL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(96), Some(96), Some(96), Some(96)]),
            },
            WhatsMinerModel::M64VL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(114), Some(114), Some(114), Some(114)]),
            },
            WhatsMinerModel::M64VL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(120), Some(120), Some(120), Some(120)]),
            },
            WhatsMinerModel::M65SPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(456), Some(456), Some(456), Some(456)]),
            },
            WhatsMinerModel::M65SPlusVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(340), Some(340), Some(340), Some(340)]),
            },
            WhatsMinerModel::M65SVK20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(350), Some(350), Some(350), Some(350)]),
            },
            WhatsMinerModel::M65SVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M66SPlusPlusVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(368), Some(368), Some(368)]),
            },
            WhatsMinerModel::M66SPlusPlusVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(288), Some(288), Some(288)]),
            },
            WhatsMinerModel::M66SPlusPlusVL50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M66SPlusPlusVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(250), Some(250), Some(250), Some(250)]),
            },
            WhatsMinerModel::M66SPlusPlusVL70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(250), Some(250), Some(250), Some(250)]),
            },
            WhatsMinerModel::M66SPlusPlusVL80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(230), Some(230), Some(230), Some(230)]),
            },
            WhatsMinerModel::M66SPlusPlusVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(138), Some(138), Some(138), Some(138)]),
            },
            WhatsMinerModel::M66SPlusVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(440), Some(440), Some(440)]),
            },
            WhatsMinerModel::M66SPlusVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(220), Some(220), Some(220), Some(220)]),
            },
            WhatsMinerModel::M66SPlusVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(230), Some(230), Some(230), Some(230)]),
            },
            WhatsMinerModel::M66SPlusVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M66SPlusVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(250), Some(250), Some(250), Some(250)]),
            },
            WhatsMinerModel::M66SPlusVL50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(200), Some(200), Some(200), Some(200)]),
            },
            WhatsMinerModel::M66SPlusVL60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(200), Some(200), Some(200), Some(200)]),
            },
            WhatsMinerModel::M66SPlusVL70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(230), Some(230), Some(230), Some(230)]),
            },
            WhatsMinerModel::M66SPlusVL80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(250), Some(250), Some(250), Some(250)]),
            },
            WhatsMinerModel::M66SVK20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(368), Some(368), Some(368)]),
            },
            WhatsMinerModel::M66SVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(384), Some(384), Some(384)]),
            },
            WhatsMinerModel::M66SVK40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M66SVK50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(250), Some(250), Some(250), Some(250)]),
            },
            WhatsMinerModel::M66SVK60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(250), Some(250), Some(250), Some(250)]),
            },
            WhatsMinerModel::M66SVK70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(210), Some(210), Some(210), Some(210)]),
            },
            WhatsMinerModel::M66SVK80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(220), Some(220), Some(220), Some(220)]),
            },
            WhatsMinerModel::M66SVL10 => Self {
                fans: Some(0),
                boards: Some(vec![Some(168), Some(168), Some(168), Some(168)]),
            },
            WhatsMinerModel::M66SVL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(176), Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M66SVL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(192), Some(192), Some(192), Some(192)]),
            },
            WhatsMinerModel::M66SVL40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(200), Some(200), Some(200), Some(200)]),
            },
            WhatsMinerModel::M66SVL50 => Self {
                fans: Some(0),
                boards: Some(vec![Some(210), Some(210), Some(210), Some(210)]),
            },
            WhatsMinerModel::M66SVL80 => Self {
                fans: Some(0),
                boards: Some(vec![Some(160), Some(160), Some(160), Some(160)]),
            },
            WhatsMinerModel::M66VK20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(184), Some(184), Some(184), Some(184)]),
            },
            WhatsMinerModel::M66VK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(192), Some(192), Some(192), Some(192)]),
            },
            WhatsMinerModel::M66VK60 => Self {
                fans: Some(0),
                boards: Some(vec![Some(176), Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M66VL20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(160), Some(160), Some(160), Some(160)]),
            },
            WhatsMinerModel::M66VL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(168), Some(168), Some(168), Some(168)]),
            },
            WhatsMinerModel::M67SVK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(440), Some(440), Some(440)]),
            },
            WhatsMinerModel::M69SPlusPlusVM30 => Self {
                fans: Some(0),
                boards: Some(vec![
                    Some(228),
                    Some(228),
                    Some(228),
                    Some(228),
                    Some(228),
                    Some(228),
                    Some(228),
                    Some(228),
                ]),
            },
            WhatsMinerModel::M69VK30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M70SPlusVM30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M70SVM30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M70VL20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(245), Some(245), Some(245)]),
            },
            WhatsMinerModel::M70VL30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(255), Some(255), Some(255)]),
            },
            WhatsMinerModel::M70VM10 => Self {
                fans: Some(2),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            WhatsMinerModel::M70VM20 => Self {
                fans: Some(2),
                boards: Some(vec![Some(141), Some(141), Some(141)]),
            },
            WhatsMinerModel::M70VM30 => Self {
                fans: Some(2),
                boards: Some(vec![Some(147), Some(147), Some(147)]),
            },
            WhatsMinerModel::M70VM80 => Self {
                fans: Some(2),
                boards: Some(vec![Some(147), Some(147), Some(147)]),
            },
            WhatsMinerModel::M72SVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(234), Some(234), Some(234)]),
            },
            WhatsMinerModel::M72VM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(174), Some(174), Some(174)]),
            },
            WhatsMinerModel::M73SPlusVM40 => Self {
                fans: Some(0),
                boards: Some(vec![Some(380), Some(380), Some(380), Some(380)]),
            },
            WhatsMinerModel::M73SVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(304), Some(304), Some(304), Some(304)]),
            },
            WhatsMinerModel::M73VL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(380), Some(380), Some(380), Some(380)]),
            },
            WhatsMinerModel::M73VM20 => Self {
                fans: Some(0),
                boards: Some(vec![Some(216), Some(216), Some(216), Some(216)]),
            },
            WhatsMinerModel::M73VM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(228), Some(228), Some(228), Some(228)]),
            },
            WhatsMinerModel::M73VM70 => Self {
                fans: Some(0),
                boards: Some(vec![Some(204), Some(204), Some(204), Some(204)]),
            },
            WhatsMinerModel::M76SVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(240), Some(240), Some(240)]),
            },
            WhatsMinerModel::M76VL30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(384), Some(384), Some(384)]),
            },
            WhatsMinerModel::M76VM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(176), Some(176), Some(176)]),
            },
            WhatsMinerModel::M78SVM30 => Self {
                fans: Some(0),
                boards: Some(vec![Some(384), Some(384), Some(384)]),
            },
            WhatsMinerModel::M79SVM30 => Self {
                fans: Some(0),
                boards: Some(vec![
                    Some(304),
                    Some(304),
                    Some(304),
                    Some(304),
                    Some(304),
                    Some(304),
                    Some(304),
                    Some(304),
                ]),
            },
            WhatsMinerModel::Unknown(_) => Default::default(),
        }
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum WhatsMinerControlBoard {
    #[serde(rename = "H3")]
    H3,
    #[serde(rename = "H6")]
    H6,
    #[serde(rename = "H6OS")]
    H6OS,
    #[serde(rename = "H616")]
    H616,
}

impl WhatsMinerControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().replace(' ', "").to_uppercase();
        match cb_model.as_ref() {
            "H3" => Some(Self::H3),
            "H6" => Some(Self::H6),
            "H6OS" => Some(Self::H6OS),
            "H616" => Some(Self::H616),
            _ => None,
        }
    }
}

impl FromValue for WhatsMinerControlBoard {
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<WhatsMinerControlBoard> for MinerControlBoard {
    fn from(cb: WhatsMinerControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}
