use crate::data::device::MinerHardware;
use crate::data::device::models::antminer::AntMinerModel;
use crate::data::device::models::avalon::AvalonMinerModel;
use crate::data::device::models::bitaxe::BitaxeModel;
use crate::data::device::models::braiins::BraiinsModel;
use crate::data::device::models::epic::EPicModel;
use crate::data::device::models::nerdaxe::NerdAxeModel;
use crate::data::device::models::whatsminer::WhatsMinerModel;

impl From<&AvalonMinerModel> for MinerHardware {
    fn from(value: &AvalonMinerModel) -> Self {
        match value {
            AvalonMinerModel::Avalon721 => Self {
                boards: Some(4),
                chips: Some(18),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon741 => Self {
                boards: Some(4),
                chips: Some(22),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon761 => Self {
                boards: Some(4),
                chips: Some(18),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon821 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon841 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon851 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon921 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon1026 => Self {
                boards: Some(3),
                chips: Some(80),
                fans: Some(2),
            },
            AvalonMinerModel::Avalon1047 => Self {
                boards: Some(3),
                chips: Some(80),
                fans: Some(2),
            },
            AvalonMinerModel::Avalon1066 => Self {
                boards: Some(3),
                chips: Some(114),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1126Pro => Self {
                boards: Some(3),
                chips: Some(120),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1166Pro => Self {
                boards: Some(3),
                chips: Some(120),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1246 => Self {
                boards: Some(3),
                chips: Some(120),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1566 => Self {
                boards: Some(3),
                chips: Some(160),
                fans: Some(2),
            },
            AvalonMinerModel::AvalonNano3 => Self {
                boards: Some(1),
                chips: Some(10),
                fans: Some(1),
            },
            AvalonMinerModel::AvalonNano3s => Self {
                boards: Some(1),
                chips: Some(12),
                fans: Some(1),
            },
            AvalonMinerModel::AvalonHomeQ => Self {
                boards: Some(1),
                chips: Some(160),
                fans: Some(4),
            },
        }
    }
}

impl From<&WhatsMinerModel> for MinerHardware {
    fn from(value: &WhatsMinerModel) -> Self {
        match value {
            WhatsMinerModel::M20PV10 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M20PV30 => Self {
                chips: Some(148),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M20SPlusV30 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M20SV10 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M20SV20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M20SV30 => Self {
                chips: Some(140),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M20V10 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M21SPlusV20 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M21SV20 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M21SV60 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M21SV70 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M21V10 => Self {
                chips: Some(33),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M29V10 => Self {
                chips: Some(50),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30KV10 => Self {
                chips: Some(240),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M30LV10 => Self {
                chips: Some(144),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M30SPlusPlusV10 => Self {
                chips: Some(255),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M30SPlusPlusV20 => Self {
                chips: Some(255),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M30SPlusPlusVE30 => Self {
                chips: Some(215),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVE40 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVE50 => Self {
                chips: Some(235),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVF40 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVG30 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVG40 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVG50 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH10 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH100 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH110 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH20 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH30 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH40 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH50 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH60 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH70 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH80 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVH90 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVI30 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVJ20 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVJ30 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVJ50 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVJ60 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVJ70 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusPlusVK30 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(2),
            },
            WhatsMinerModel::M30SPlusPlusVK40 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV10 => Self {
                chips: Some(215),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV100 => Self {
                chips: Some(215),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV20 => Self {
                chips: Some(255),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV30 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV40 => Self {
                chips: Some(235),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV50 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV60 => Self {
                chips: Some(245),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV70 => Self {
                chips: Some(235),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV80 => Self {
                chips: Some(245),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusV90 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE100 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE30 => Self {
                chips: Some(148),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE40 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE50 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE60 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE70 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE80 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVE90 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVF20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVF30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVG20 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVG30 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVG40 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVG50 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVG60 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH10 => Self {
                chips: Some(64),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH20 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH30 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH40 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH50 => Self {
                chips: Some(64),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH60 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVH70 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVI30 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVJ30 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SPlusVJ40 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV10 => Self {
                chips: Some(148),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV20 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV30 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV40 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV50 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV60 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV70 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SV80 => Self {
                chips: Some(129),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE10 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE40 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE50 => Self {
                chips: Some(129),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE60 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVE70 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVF10 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVF20 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVF30 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVG10 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVG20 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVG30 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVG40 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVH10 => Self {
                chips: Some(64),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVH20 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVH30 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVH40 => Self {
                chips: Some(64),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVH50 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVH60 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVI20 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30SVJ30 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30V10 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M30V20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31HV10 => Self {
                chips: Some(114),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31HV40 => Self {
                chips: Some(136),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M31LV10 => Self {
                chips: Some(144),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M31SPlusV10 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV100 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV40 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV50 => Self {
                chips: Some(148),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV60 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV80 => Self {
                chips: Some(129),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusV90 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE10 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE20 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE30 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE40 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE50 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE60 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVE80 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVF20 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVF30 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVG20 => Self {
                chips: Some(66),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SPlusVG30 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SEV10 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SEV20 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SEV30 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV10 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV40 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV50 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV60 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV70 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV80 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SV90 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SVE10 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SVE20 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31SVE30 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31V10 => Self {
                chips: Some(70),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M31V20 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M32V10 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M32V20 => Self {
                chips: Some(74),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M33SPlusPlusVG40 => Self {
                chips: Some(174),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SPlusPlusVH20 => Self {
                chips: Some(112),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SPlusPlusVH30 => Self {
                chips: None,
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SPlusVG20 => Self {
                chips: Some(112),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SPlusVG30 => Self {
                chips: Some(162),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SPlusVH20 => Self {
                chips: Some(100),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SPlusVH30 => Self {
                chips: None,
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33SVG30 => Self {
                chips: Some(116),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M33V10 => Self {
                chips: Some(33),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M33V20 => Self {
                chips: Some(62),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M33V30 => Self {
                chips: Some(66),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M34SPlusVE10 => Self {
                chips: Some(116),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M36SPlusPlusVH30 => Self {
                chips: Some(80),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M36SPlusVG30 => Self {
                chips: Some(108),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M36SVE10 => Self {
                chips: Some(114),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M39V10 => Self {
                chips: Some(50),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M39V20 => Self {
                chips: Some(54),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M39V30 => Self {
                chips: Some(68),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVK10 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVK20 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVK30 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVK40 => Self {
                chips: Some(129),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVK50 => Self {
                chips: Some(135),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVK60 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVL20 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVL30 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVL40 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVL50 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusPlusVL60 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVH30 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVH40 => Self {
                chips: Some(180),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVJ30 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVJ40 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVJ60 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVK10 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVK20 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVK30 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVL10 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVL20 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SPlusVL30 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVH10 => Self {
                chips: None,
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVH20 => Self {
                chips: Some(135),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVH30 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVH40 => Self {
                chips: Some(148),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVH50 => Self {
                chips: Some(135),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVJ10 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVJ20 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVJ30 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVJ40 => Self {
                chips: Some(129),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVJ50 => Self {
                chips: Some(135),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK10 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK50 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK60 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK70 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVK80 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVL20 => Self {
                chips: Some(78),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50SVL30 => Self {
                chips: Some(82),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VE30 => Self {
                chips: Some(255),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M50VG30 => Self {
                chips: Some(156),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH10 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH40 => Self {
                chips: Some(84),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH50 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH60 => Self {
                chips: Some(84),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH70 => Self {
                chips: Some(105),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH80 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VH90 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VJ10 => Self {
                chips: Some(86),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VJ20 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VJ30 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VJ40 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VJ60 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VK40 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M50VK50 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M52SPlusPlusVL10 => Self {
                chips: Some(87),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M52SVK30 => Self {
                chips: Some(62),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53HVH10 => Self {
                chips: Some(56),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusPlusVK10 => Self {
                chips: Some(198),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusPlusVK20 => Self {
                chips: Some(192),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusPlusVK30 => Self {
                chips: Some(240),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusPlusVK50 => Self {
                chips: Some(186),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusPlusVL10 => Self {
                chips: Some(128),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusPlusVL30 => Self {
                chips: Some(174),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusVJ30 => Self {
                chips: Some(240),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusVJ40 => Self {
                chips: Some(248),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusVJ50 => Self {
                chips: Some(264),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SPlusVK30 => Self {
                chips: Some(168),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SVH20 => Self {
                chips: Some(198),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SVH30 => Self {
                chips: Some(204),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SVJ30 => Self {
                chips: Some(180),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SVJ40 => Self {
                chips: Some(192),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53SVK30 => Self {
                chips: Some(128),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53VH30 => Self {
                chips: Some(128),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53VH40 => Self {
                chips: Some(174),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53VH50 => Self {
                chips: Some(162),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53VK30 => Self {
                chips: Some(100),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M53VK60 => Self {
                chips: Some(100),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M54SPlusPlusVK30 => Self {
                chips: Some(96),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M54SPlusPlusVL30 => Self {
                chips: Some(68),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M54SPlusPlusVL40 => Self {
                chips: Some(90),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusPlusVK10 => Self {
                chips: Some(160),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusPlusVK30 => Self {
                chips: Some(176),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusPlusVK40 => Self {
                chips: Some(132),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusPlusVK50 => Self {
                chips: Some(152),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusVJ30 => Self {
                chips: Some(176),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusVK30 => Self {
                chips: Some(108),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusVK40 => Self {
                chips: Some(114),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SPlusVK50 => Self {
                chips: Some(120),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SVH30 => Self {
                chips: Some(152),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SVJ30 => Self {
                chips: Some(132),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56SVJ40 => Self {
                chips: Some(152),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M56VH30 => Self {
                chips: Some(108),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M59VH30 => Self {
                chips: Some(132),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M60SPlusPlusVL30 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusPlusVL40 => Self {
                chips: Some(235),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVK30 => Self {
                chips: Some(245),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVK40 => Self {
                chips: Some(215),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M60SPlusVK50 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(4),
            },
            WhatsMinerModel::M60SPlusVK60 => Self {
                chips: Some(294),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVK70 => Self {
                chips: Some(306),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVL10 => Self {
                chips: Some(196),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVL30 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVL40 => Self {
                chips: Some(188),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVL50 => Self {
                chips: Some(180),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SPlusVL60 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVK10 => Self {
                chips: Some(215),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVK20 => Self {
                chips: Some(235),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVK30 => Self {
                chips: Some(245),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVK40 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL10 => Self {
                chips: Some(147),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL20 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL30 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL40 => Self {
                chips: Some(180),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL50 => Self {
                chips: Some(188),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL60 => Self {
                chips: Some(196),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60SVL70 => Self {
                chips: Some(141),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VK10 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VK20 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VK30 => Self {
                chips: Some(215),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VK40 => Self {
                chips: Some(180),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VK6A => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VL10 => Self {
                chips: Some(111),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VL20 => Self {
                chips: Some(117),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VL30 => Self {
                chips: Some(123),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VL40 => Self {
                chips: Some(129),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M60VL50 => Self {
                chips: Some(135),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61SPlusVL30 => Self {
                chips: Some(225),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61SVL10 => Self {
                chips: Some(164),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61SVL20 => Self {
                chips: Some(172),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61SVL30 => Self {
                chips: Some(180),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VK10 => Self {
                chips: Some(180),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VK20 => Self {
                chips: Some(184),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VK30 => Self {
                chips: Some(188),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VK40 => Self {
                chips: Some(192),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VL10 => Self {
                chips: Some(135),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VL30 => Self {
                chips: Some(141),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VL40 => Self {
                chips: Some(144),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VL50 => Self {
                chips: Some(147),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M61VL60 => Self {
                chips: Some(150),
                fans: Some(2),
                boards: Some(3),
            },
            WhatsMinerModel::M62SPlusVK30 => Self {
                chips: Some(430),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M63SPlusPlusVL20 => Self {
                chips: Some(380),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SPlusVK30 => Self {
                chips: Some(456),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SPlusVL10 => Self {
                chips: Some(304),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SPlusVL20 => Self {
                chips: Some(340),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SPlusVL30 => Self {
                chips: Some(370),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SPlusVL50 => Self {
                chips: Some(272),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVK10 => Self {
                chips: Some(340),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVK20 => Self {
                chips: Some(350),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVK30 => Self {
                chips: Some(370),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVK60 => Self {
                chips: Some(350),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVL10 => Self {
                chips: Some(228),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVL50 => Self {
                chips: Some(288),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63SVL60 => Self {
                chips: Some(288),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63VK10 => Self {
                chips: None,
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63VK20 => Self {
                chips: Some(264),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63VK30 => Self {
                chips: Some(272),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63VL10 => Self {
                chips: Some(174),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M63VL30 => Self {
                chips: Some(216),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M64SVL30 => Self {
                chips: Some(152),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M64VL30 => Self {
                chips: Some(114),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M64VL40 => Self {
                chips: Some(120),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M65SPlusVK30 => Self {
                chips: Some(456),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M65SVK20 => Self {
                chips: Some(350),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M65SVL60 => Self {
                chips: Some(288),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SPlusPlusVL20 => Self {
                chips: Some(368),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M66SPlusVK30 => Self {
                chips: Some(440),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M66SPlusVL10 => Self {
                chips: Some(220),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SPlusVL20 => Self {
                chips: Some(230),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SPlusVL30 => Self {
                chips: Some(240),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SPlusVL40 => Self {
                chips: Some(250),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SPlusVL60 => Self {
                chips: Some(200),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVK20 => Self {
                chips: Some(368),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M66SVK30 => Self {
                chips: Some(384),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M66SVK40 => Self {
                chips: Some(240),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVK50 => Self {
                chips: Some(250),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVK60 => Self {
                chips: Some(250),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVL10 => Self {
                chips: Some(168),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVL20 => Self {
                chips: Some(176),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVL30 => Self {
                chips: Some(192),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVL40 => Self {
                chips: Some(200),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66SVL50 => Self {
                chips: Some(210),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66VK20 => Self {
                chips: Some(184),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66VK30 => Self {
                chips: Some(192),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66VL20 => Self {
                chips: Some(160),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M66VL30 => Self {
                chips: Some(168),
                fans: Some(0),
                boards: Some(4),
            },
            WhatsMinerModel::M67SVK30 => Self {
                chips: Some(440),
                fans: Some(0),
                boards: Some(3),
            },
            WhatsMinerModel::M70VM30 => Self {
                chips: Some(147),
                fans: Some(2),
                boards: Some(3),
            },
        }
    }
}

impl From<&BitaxeModel> for MinerHardware {
    fn from(_model: &BitaxeModel) -> Self {
        Self {
            chips: Some(1),
            fans: Some(1),
            boards: Some(1),
        }
    }
}

impl From<&NerdAxeModel> for MinerHardware {
    fn from(model: &NerdAxeModel) -> Self {
        match model {
            NerdAxeModel::NerdAxe => Self {
                chips: Some(1),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::NerdQAxe => Self {
                chips: Some(4),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::NerdMiner => Self {
                chips: Some(1),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::NerdAxeUltra => Self {
                chips: Some(1),
                fans: Some(1),
                boards: Some(1),
            },
        }
    }
}

impl From<&BraiinsModel> for MinerHardware {
    fn from(value: &BraiinsModel) -> Self {
        match value {
            BraiinsModel::BMM100 => Self {
                chips: None,
                fans: Some(1),
                boards: Some(1),
            },
            BraiinsModel::BMM101 => Self {
                chips: None,
                fans: Some(1),
                boards: Some(1),
            },
        }
    }
}

impl From<&EPicModel> for MinerHardware {
    fn from(value: &EPicModel) -> Self {
        match value {
            EPicModel::BM520i => Self {
                chips: Some(124),
                fans: Some(4),
                boards: Some(3),
            },
            EPicModel::S19JProDual => Self {
                chips: Some(126),
                fans: Some(8),
                boards: Some(6),
            },
        }
    }
}

impl From<&AntMinerModel> for MinerHardware {
    fn from(value: &AntMinerModel) -> Self {
        match value {
            AntMinerModel::D3 => Self {
                chips: Some(60),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::HS3 => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::L3Plus => Self {
                chips: Some(72),
                fans: Some(2),
                boards: Some(4),
            },
            AntMinerModel::KA3 => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::KS3 => Self {
                chips: Some(92),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::DR5 => Self {
                chips: Some(72),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::KS5 => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::KS5Pro => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::L7 => Self {
                chips: Some(120),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::K7 => Self {
                chips: Some(92),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::D7 => Self {
                chips: Some(70),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::E9Pro => Self {
                chips: Some(8),
                fans: Some(4),
                boards: Some(2),
            },
            AntMinerModel::D9 => Self {
                chips: Some(126),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S9 => Self {
                chips: Some(63),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::S9i => Self {
                chips: Some(63),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::S9j => Self {
                chips: Some(63),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::T9 => Self {
                chips: Some(54),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::L9 => Self {
                chips: Some(110),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::Z15 => Self {
                chips: Some(3),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::Z15Pro => Self {
                chips: Some(6),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::S17 => Self {
                chips: Some(48),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S17Plus => Self {
                chips: Some(65),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S17Pro => Self {
                chips: Some(48),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S17e => Self {
                chips: Some(135),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T17 => Self {
                chips: Some(30),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T17Plus => Self {
                chips: Some(44),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T17e => Self {
                chips: Some(78),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19 => Self {
                chips: Some(76),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19L => Self {
                chips: Some(76),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19Pro => Self {
                chips: Some(114),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19j => Self {
                chips: Some(114),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19i => Self {
                chips: Some(80),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19Plus => Self {
                chips: Some(80),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jNoPIC => Self {
                chips: Some(88),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19ProPlus => Self {
                chips: Some(120),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jPro => Self {
                chips: Some(126),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jProPlus => Self {
                chips: Some(120),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19XP => Self {
                chips: Some(110),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19a => Self {
                chips: Some(72),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19aPro => Self {
                chips: Some(100),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19Hydro => Self {
                chips: Some(104),
                fans: Some(0),
                boards: Some(4),
            },
            AntMinerModel::S19ProHydro => Self {
                chips: Some(180),
                fans: Some(0),
                boards: Some(4),
            },
            AntMinerModel::S19ProPlusHydro => Self {
                chips: Some(180),
                fans: Some(0),
                boards: Some(4),
            },
            AntMinerModel::S19KPro => Self {
                chips: Some(77),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jXP => Self {
                chips: Some(110),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T19 => Self {
                chips: Some(76),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21 => Self {
                chips: Some(108),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21Plus => Self {
                chips: Some(55),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21PlusHydro => Self {
                chips: Some(95),
                fans: Some(0),
                boards: Some(3),
            },
            AntMinerModel::S21Pro => Self {
                chips: Some(65),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21XP => Self {
                chips: Some(91),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T21 => Self {
                chips: Some(108),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21Hydro => Self {
                chips: Some(216),
                fans: Some(0),
                boards: Some(3),
            },
            AntMinerModel::S21eXPHydro => Self {
                chips: Some(160),
                fans: Some(0),
                boards: Some(3),
            },
        }
    }
}
