#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum WhatsMinerModel {
    #[serde(alias = "M20PV10")]
    M20PV10,
    #[serde(alias = "M20PV30")]
    M20PV30,
    #[serde(alias = "M20S+V30")]
    M20SPlusV30,
    #[serde(alias = "M20SV10")]
    M20SV10,
    #[serde(alias = "M20SV20")]
    M20SV20,
    #[serde(alias = "M20SV30")]
    M20SV30,
    #[serde(alias = "M20V10")]
    M20V10,
    #[serde(alias = "M21S+V20")]
    M21SPlusV20,
    #[serde(alias = "M21SV20")]
    M21SV20,
    #[serde(alias = "M21SV60")]
    M21SV60,
    #[serde(alias = "M21SV70")]
    M21SV70,
    #[serde(alias = "M21V10")]
    M21V10,
    #[serde(alias = "M29V10")]
    M29V10,
    #[serde(alias = "M30KV10")]
    M30KV10,
    #[serde(alias = "M30LV10")]
    M30LV10,
    #[serde(alias = "M30S++V10")]
    M30SPlusPlusV10,
    #[serde(alias = "M30S++V20")]
    M30SPlusPlusV20,
    #[serde(alias = "M30S++VE30")]
    M30SPlusPlusVE30,
    #[serde(alias = "M30S++VE40")]
    M30SPlusPlusVE40,
    #[serde(alias = "M30S++VE50")]
    M30SPlusPlusVE50,
    #[serde(alias = "M30S++VF40")]
    M30SPlusPlusVF40,
    #[serde(alias = "M30S++VG30")]
    M30SPlusPlusVG30,
    #[serde(alias = "M30S++VG40")]
    M30SPlusPlusVG40,
    #[serde(alias = "M30S++VG50")]
    M30SPlusPlusVG50,
    #[serde(alias = "M30S++VH10")]
    M30SPlusPlusVH10,
    #[serde(alias = "M30S++VH100")]
    M30SPlusPlusVH100,
    #[serde(alias = "M30S++VH110")]
    M30SPlusPlusVH110,
    #[serde(alias = "M30S++VH20")]
    M30SPlusPlusVH20,
    #[serde(alias = "M30S++VH30")]
    M30SPlusPlusVH30,
    #[serde(alias = "M30S++VH40")]
    M30SPlusPlusVH40,
    #[serde(alias = "M30S++VH50")]
    M30SPlusPlusVH50,
    #[serde(alias = "M30S++VH60")]
    M30SPlusPlusVH60,
    #[serde(alias = "M30S++VH70")]
    M30SPlusPlusVH70,
    #[serde(alias = "M30S++VH80")]
    M30SPlusPlusVH80,
    #[serde(alias = "M30S++VH90")]
    M30SPlusPlusVH90,
    #[serde(alias = "M30S++VHA0")]
    M30SPlusPlusVHA0,
    #[serde(alias = "M30S++VHB0")]
    M30SPlusPlusVHB0,
    #[serde(alias = "M30S++VI30")]
    M30SPlusPlusVI30,
    #[serde(alias = "M30S++VJ20")]
    M30SPlusPlusVJ20,
    #[serde(alias = "M30S++VJ30")]
    M30SPlusPlusVJ30,
    #[serde(alias = "M30S++VJ50")]
    M30SPlusPlusVJ50,
    #[serde(alias = "M30S++VJ60")]
    M30SPlusPlusVJ60,
    #[serde(alias = "M30S++VJ70")]
    M30SPlusPlusVJ70,
    #[serde(alias = "M30S++VK20")]
    M30SPlusPlusVK20,
    #[serde(alias = "M30S++VK30")]
    M30SPlusPlusVK30,
    #[serde(alias = "M30S++VK40")]
    M30SPlusPlusVK40,
    #[serde(alias = "M30S+V10")]
    M30SPlusV10,
    #[serde(alias = "M30S+V100")]
    M30SPlusV100,
    #[serde(alias = "M30S+V20")]
    M30SPlusV20,
    #[serde(alias = "M30S+V30")]
    M30SPlusV30,
    #[serde(alias = "M30S+V40")]
    M30SPlusV40,
    #[serde(alias = "M30S+V50")]
    M30SPlusV50,
    #[serde(alias = "M30S+V60")]
    M30SPlusV60,
    #[serde(alias = "M30S+V70")]
    M30SPlusV70,
    #[serde(alias = "M30S+V80")]
    M30SPlusV80,
    #[serde(alias = "M30S+V90")]
    M30SPlusV90,
    #[serde(alias = "M30S+VA0")]
    M30SPlusVA0,
    #[serde(alias = "M30S+VE100")]
    M30SPlusVE100,
    #[serde(alias = "M30S+VE30")]
    M30SPlusVE30,
    #[serde(alias = "M30S+VE40")]
    M30SPlusVE40,
    #[serde(alias = "M30S+VE50")]
    M30SPlusVE50,
    #[serde(alias = "M30S+VE60")]
    M30SPlusVE60,
    #[serde(alias = "M30S+VE70")]
    M30SPlusVE70,
    #[serde(alias = "M30S+VE80")]
    M30SPlusVE80,
    #[serde(alias = "M30S+VE90")]
    M30SPlusVE90,
    #[serde(alias = "M30S+VF20")]
    M30SPlusVF20,
    #[serde(alias = "M30S+VF30")]
    M30SPlusVF30,
    #[serde(alias = "M30S+VG20")]
    M30SPlusVG20,
    #[serde(alias = "M30S+VG30")]
    M30SPlusVG30,
    #[serde(alias = "M30S+VG40")]
    M30SPlusVG40,
    #[serde(alias = "M30S+VG50")]
    M30SPlusVG50,
    #[serde(alias = "M30S+VG60")]
    M30SPlusVG60,
    #[serde(alias = "M30S+VH10")]
    M30SPlusVH10,
    #[serde(alias = "M30S+VH20")]
    M30SPlusVH20,
    #[serde(alias = "M30S+VH30")]
    M30SPlusVH30,
    #[serde(alias = "M30S+VH40")]
    M30SPlusVH40,
    #[serde(alias = "M30S+VH50")]
    M30SPlusVH50,
    #[serde(alias = "M30S+VH60")]
    M30SPlusVH60,
    #[serde(alias = "M30S+VH70")]
    M30SPlusVH70,
    #[serde(alias = "M30S+VI30")]
    M30SPlusVI30,
    #[serde(alias = "M30S+VJ30")]
    M30SPlusVJ30,
    #[serde(alias = "M30S+VJ40")]
    M30SPlusVJ40,
    #[serde(alias = "M30SV10")]
    M30SV10,
    #[serde(alias = "M30SV20")]
    M30SV20,
    #[serde(alias = "M30SV30")]
    M30SV30,
    #[serde(alias = "M30SV40")]
    M30SV40,
    #[serde(alias = "M30SV50")]
    M30SV50,
    #[serde(alias = "M30SV60")]
    M30SV60,
    #[serde(alias = "M30SV70")]
    M30SV70,
    #[serde(alias = "M30SV80")]
    M30SV80,
    #[serde(alias = "M30SVE10")]
    M30SVE10,
    #[serde(alias = "M30SVE20")]
    M30SVE20,
    #[serde(alias = "M30SVE30")]
    M30SVE30,
    #[serde(alias = "M30SVE40")]
    M30SVE40,
    #[serde(alias = "M30SVE50")]
    M30SVE50,
    #[serde(alias = "M30SVE60")]
    M30SVE60,
    #[serde(alias = "M30SVE70")]
    M30SVE70,
    #[serde(alias = "M30SVF10")]
    M30SVF10,
    #[serde(alias = "M30SVF20")]
    M30SVF20,
    #[serde(alias = "M30SVF30")]
    M30SVF30,
    #[serde(alias = "M30SVG10")]
    M30SVG10,
    #[serde(alias = "M30SVG20")]
    M30SVG20,
    #[serde(alias = "M30SVG30")]
    M30SVG30,
    #[serde(alias = "M30SVG40")]
    M30SVG40,
    #[serde(alias = "M30SVH10")]
    M30SVH10,
    #[serde(alias = "M30SVH20")]
    M30SVH20,
    #[serde(alias = "M30SVH30")]
    M30SVH30,
    #[serde(alias = "M30SVH40")]
    M30SVH40,
    #[serde(alias = "M30SVH50")]
    M30SVH50,
    #[serde(alias = "M30SVH60")]
    M30SVH60,
    #[serde(alias = "M30SVI20")]
    M30SVI20,
    #[serde(alias = "M30SVJ30")]
    M30SVJ30,
    #[serde(alias = "M30V10")]
    M30V10,
    #[serde(alias = "M30V20")]
    M30V20,
    #[serde(alias = "M31HV10")]
    M31HV10,
    #[serde(alias = "M31HV40")]
    M31HV40,
    #[serde(alias = "M31LV10")]
    M31LV10,
    #[serde(alias = "M31S+V10")]
    M31SPlusV10,
    #[serde(alias = "M31S+V100")]
    M31SPlusV100,
    #[serde(alias = "M31S+V20")]
    M31SPlusV20,
    #[serde(alias = "M31S+V30")]
    M31SPlusV30,
    #[serde(alias = "M31S+V40")]
    M31SPlusV40,
    #[serde(alias = "M31S+V50")]
    M31SPlusV50,
    #[serde(alias = "M31S+V60")]
    M31SPlusV60,
    #[serde(alias = "M31S+V80")]
    M31SPlusV80,
    #[serde(alias = "M31S+V90")]
    M31SPlusV90,
    #[serde(alias = "M31S+VA0")]
    M31SPlusVA0,
    #[serde(alias = "M31S+VE10")]
    M31SPlusVE10,
    #[serde(alias = "M31S+VE20")]
    M31SPlusVE20,
    #[serde(alias = "M31S+VE30")]
    M31SPlusVE30,
    #[serde(alias = "M31S+VE40")]
    M31SPlusVE40,
    #[serde(alias = "M31S+VE50")]
    M31SPlusVE50,
    #[serde(alias = "M31S+VE60")]
    M31SPlusVE60,
    #[serde(alias = "M31S+VE80")]
    M31SPlusVE80,
    #[serde(alias = "M31S+VF20")]
    M31SPlusVF20,
    #[serde(alias = "M31S+VF30")]
    M31SPlusVF30,
    #[serde(alias = "M31S+VG20")]
    M31SPlusVG20,
    #[serde(alias = "M31S+VG30")]
    M31SPlusVG30,
    #[serde(alias = "M31SEV10")]
    M31SEV10,
    #[serde(alias = "M31SEV20")]
    M31SEV20,
    #[serde(alias = "M31SEV30")]
    M31SEV30,
    #[serde(alias = "M31SV10")]
    M31SV10,
    #[serde(alias = "M31SV20")]
    M31SV20,
    #[serde(alias = "M31SV30")]
    M31SV30,
    #[serde(alias = "M31SV40")]
    M31SV40,
    #[serde(alias = "M31SV50")]
    M31SV50,
    #[serde(alias = "M31SV60")]
    M31SV60,
    #[serde(alias = "M31SV70")]
    M31SV70,
    #[serde(alias = "M31SV80")]
    M31SV80,
    #[serde(alias = "M31SV90")]
    M31SV90,
    #[serde(alias = "M31SVE10")]
    M31SVE10,
    #[serde(alias = "M31SVE20")]
    M31SVE20,
    #[serde(alias = "M31SVE30")]
    M31SVE30,
    #[serde(alias = "M31V10")]
    M31V10,
    #[serde(alias = "M31V20")]
    M31V20,
    #[serde(alias = "M32V10")]
    M32V10,
    #[serde(alias = "M32V20")]
    M32V20,
    #[serde(alias = "M33S++VG40")]
    M33SPlusPlusVG40,
    #[serde(alias = "M33S++VH20")]
    M33SPlusPlusVH20,
    #[serde(alias = "M33S++VH30")]
    M33SPlusPlusVH30,
    #[serde(alias = "M33S+VG20")]
    M33SPlusVG20,
    #[serde(alias = "M33S+VG30")]
    M33SPlusVG30,
    #[serde(alias = "M33S+VH20")]
    M33SPlusVH20,
    #[serde(alias = "M33S+VH30")]
    M33SPlusVH30,
    #[serde(alias = "M33SVG30")]
    M33SVG30,
    #[serde(alias = "M33V10")]
    M33V10,
    #[serde(alias = "M33V20")]
    M33V20,
    #[serde(alias = "M33V30")]
    M33V30,
    #[serde(alias = "M34S+VE10")]
    M34SPlusVE10,
    #[serde(alias = "M36S++VH30")]
    M36SPlusPlusVH30,
    #[serde(alias = "M36S+VG30")]
    M36SPlusVG30,
    #[serde(alias = "M36SVE10")]
    M36SVE10,
    #[serde(alias = "M39V10")]
    M39V10,
    #[serde(alias = "M39V20")]
    M39V20,
    #[serde(alias = "M39V30")]
    M39V30,
    #[serde(alias = "M50S++VK10")]
    M50SPlusPlusVK10,
    #[serde(alias = "M50S++VK20")]
    M50SPlusPlusVK20,
    #[serde(alias = "M50S++VK30")]
    M50SPlusPlusVK30,
    #[serde(alias = "M50S++VK40")]
    M50SPlusPlusVK40,
    #[serde(alias = "M50S++VK50")]
    M50SPlusPlusVK50,
    #[serde(alias = "M50S++VK60")]
    M50SPlusPlusVK60,
    #[serde(alias = "M50S++VL10")]
    M50SPlusPlusVL10,
    #[serde(alias = "M50S++VL20")]
    M50SPlusPlusVL20,
    #[serde(alias = "M50S++VL30")]
    M50SPlusPlusVL30,
    #[serde(alias = "M50S++VL40")]
    M50SPlusPlusVL40,
    #[serde(alias = "M50S++VL50")]
    M50SPlusPlusVL50,
    #[serde(alias = "M50S++VL60")]
    M50SPlusPlusVL60,
    #[serde(alias = "M50S+VH30")]
    M50SPlusVH30,
    #[serde(alias = "M50S+VH40")]
    M50SPlusVH40,
    #[serde(alias = "M50S+VJ30")]
    M50SPlusVJ30,
    #[serde(alias = "M50S+VJ40")]
    M50SPlusVJ40,
    #[serde(alias = "M50S+VJ60")]
    M50SPlusVJ60,
    #[serde(alias = "M50S+VK10")]
    M50SPlusVK10,
    #[serde(alias = "M50S+VK20")]
    M50SPlusVK20,
    #[serde(alias = "M50S+VK30")]
    M50SPlusVK30,
    #[serde(alias = "M50S+VL10")]
    M50SPlusVL10,
    #[serde(alias = "M50S+VL20")]
    M50SPlusVL20,
    #[serde(alias = "M50S+VL30")]
    M50SPlusVL30,
    #[serde(alias = "M50SVH10")]
    M50SVH10,
    #[serde(alias = "M50SVH20")]
    M50SVH20,
    #[serde(alias = "M50SVH30")]
    M50SVH30,
    #[serde(alias = "M50SVH40")]
    M50SVH40,
    #[serde(alias = "M50SVH50")]
    M50SVH50,
    #[serde(alias = "M50SVJ10")]
    M50SVJ10,
    #[serde(alias = "M50SVJ20")]
    M50SVJ20,
    #[serde(alias = "M50SVJ30")]
    M50SVJ30,
    #[serde(alias = "M50SVJ40")]
    M50SVJ40,
    #[serde(alias = "M50SVJ50")]
    M50SVJ50,
    #[serde(alias = "M50SVK10")]
    M50SVK10,
    #[serde(alias = "M50SVK20")]
    M50SVK20,
    #[serde(alias = "M50SVK30")]
    M50SVK30,
    #[serde(alias = "M50SVK50")]
    M50SVK50,
    #[serde(alias = "M50SVK60")]
    M50SVK60,
    #[serde(alias = "M50SVK70")]
    M50SVK70,
    #[serde(alias = "M50SVK80")]
    M50SVK80,
    #[serde(alias = "M50SVL10")]
    M50SVL10,
    #[serde(alias = "M50SVL20")]
    M50SVL20,
    #[serde(alias = "M50SVL30")]
    M50SVL30,
    #[serde(alias = "M50VE30")]
    M50VE30,
    #[serde(alias = "M50VG30")]
    M50VG30,
    #[serde(alias = "M50VH10")]
    M50VH10,
    #[serde(alias = "M50VH20")]
    M50VH20,
    #[serde(alias = "M50VH30")]
    M50VH30,
    #[serde(alias = "M50VH40")]
    M50VH40,
    #[serde(alias = "M50VH50")]
    M50VH50,
    #[serde(alias = "M50VH60")]
    M50VH60,
    #[serde(alias = "M50VH70")]
    M50VH70,
    #[serde(alias = "M50VH80")]
    M50VH80,
    #[serde(alias = "M50VH90")]
    M50VH90,
    #[serde(alias = "M50VJ10")]
    M50VJ10,
    #[serde(alias = "M50VJ20")]
    M50VJ20,
    #[serde(alias = "M50VJ30")]
    M50VJ30,
    #[serde(alias = "M50VJ40")]
    M50VJ40,
    #[serde(alias = "M50VJ60")]
    M50VJ60,
    #[serde(alias = "M50VK40")]
    M50VK40,
    #[serde(alias = "M50VK50")]
    M50VK50,
    #[serde(alias = "M51S+VL30")]
    M51SPlusVL30,
    #[serde(alias = "M52S++VL10")]
    M52SPlusPlusVL10,
    #[serde(alias = "M52SVK30")]
    M52SVK30,
    #[serde(alias = "M53HVH10")]
    M53HVH10,
    #[serde(alias = "M53S++VK10")]
    M53SPlusPlusVK10,
    #[serde(alias = "M53S++VK20")]
    M53SPlusPlusVK20,
    #[serde(alias = "M53S++VK30")]
    M53SPlusPlusVK30,
    #[serde(alias = "M53S++VK50")]
    M53SPlusPlusVK50,
    #[serde(alias = "M53S++VK70")]
    M53SPlusPlusVK70,
    #[serde(alias = "M53S++VL10")]
    M53SPlusPlusVL10,
    #[serde(alias = "M53S++VL30")]
    M53SPlusPlusVL30,
    #[serde(alias = "M53S++VL40")]
    M53SPlusPlusVL40,
    #[serde(alias = "M53S++VL60")]
    M53SPlusPlusVL60,
    #[serde(alias = "M53S++VL80")]
    M53SPlusPlusVL80,
    #[serde(alias = "M53S+VJ30")]
    M53SPlusVJ30,
    #[serde(alias = "M53S+VJ40")]
    M53SPlusVJ40,
    #[serde(alias = "M53S+VJ50")]
    M53SPlusVJ50,
    #[serde(alias = "M53S+VK30")]
    M53SPlusVK30,
    #[serde(alias = "M53SVH20")]
    M53SVH20,
    #[serde(alias = "M53SVH30")]
    M53SVH30,
    #[serde(alias = "M53SVH40")]
    M53SVH40,
    #[serde(alias = "M53SVJ30")]
    M53SVJ30,
    #[serde(alias = "M53SVJ40")]
    M53SVJ40,
    #[serde(alias = "M53SVJ50")]
    M53SVJ50,
    #[serde(alias = "M53SVK30")]
    M53SVK30,
    #[serde(alias = "M53VH30")]
    M53VH30,
    #[serde(alias = "M53VH40")]
    M53VH40,
    #[serde(alias = "M53VH50")]
    M53VH50,
    #[serde(alias = "M53VK30")]
    M53VK30,
    #[serde(alias = "M53VK60")]
    M53VK60,
    #[serde(alias = "M54S++VK30")]
    M54SPlusPlusVK30,
    #[serde(alias = "M54S++VL30")]
    M54SPlusPlusVL30,
    #[serde(alias = "M54S++VL40")]
    M54SPlusPlusVL40,
    #[serde(alias = "M54S+VL30")]
    M54SPlusVL30,
    #[serde(alias = "M54SVH30")]
    M54SVH30,
    #[serde(alias = "M54SVK30")]
    M54SVK30,
    #[serde(alias = "M56S++VK10")]
    M56SPlusPlusVK10,
    #[serde(alias = "M56S++VK30")]
    M56SPlusPlusVK30,
    #[serde(alias = "M56S++VK40")]
    M56SPlusPlusVK40,
    #[serde(alias = "M56S++VK50")]
    M56SPlusPlusVK50,
    #[serde(alias = "M56S+VJ30")]
    M56SPlusVJ30,
    #[serde(alias = "M56S+VK30")]
    M56SPlusVK30,
    #[serde(alias = "M56S+VK40")]
    M56SPlusVK40,
    #[serde(alias = "M56S+VK50")]
    M56SPlusVK50,
    #[serde(alias = "M56SVH30")]
    M56SVH30,
    #[serde(alias = "M56SVJ30")]
    M56SVJ30,
    #[serde(alias = "M56SVJ40")]
    M56SVJ40,
    #[serde(alias = "M56VH30")]
    M56VH30,
    #[serde(alias = "M59VH30")]
    M59VH30,
    #[serde(alias = "M60S++VL10")]
    M60SPlusPlusVL10,
    #[serde(alias = "M60S++VL20")]
    M60SPlusPlusVL20,
    #[serde(alias = "M60S++VL30")]
    M60SPlusPlusVL30,
    #[serde(alias = "M60S++VL40")]
    M60SPlusPlusVL40,
    #[serde(alias = "M60S++VL50")]
    M60SPlusPlusVL50,
    #[serde(alias = "M60S++VL60")]
    M60SPlusPlusVL60,
    #[serde(alias = "M60S++VL70")]
    M60SPlusPlusVL70,
    #[serde(alias = "M60S++VL80")]
    M60SPlusPlusVL80,
    #[serde(alias = "M60S++VL90")]
    M60SPlusPlusVL90,
    #[serde(alias = "M60S++VLA0")]
    M60SPlusPlusVLA0,
    #[serde(alias = "M60S++VLB0")]
    M60SPlusPlusVLB0,
    #[serde(alias = "M60S++VM30")]
    M60SPlusPlusVM30,
    #[serde(alias = "M60S++VM40")]
    M60SPlusPlusVM40,
    #[serde(alias = "M60S++VM50")]
    M60SPlusPlusVM50,
    #[serde(alias = "M60S++VM60")]
    M60SPlusPlusVM60,
    #[serde(alias = "M60S++VM70")]
    M60SPlusPlusVM70,
    #[serde(alias = "M60S+VK30")]
    M60SPlusVK30,
    #[serde(alias = "M60S+VK40")]
    M60SPlusVK40,
    #[serde(alias = "M60S+VK50")]
    M60SPlusVK50,
    #[serde(alias = "M60S+VK60")]
    M60SPlusVK60,
    #[serde(alias = "M60S+VK70")]
    M60SPlusVK70,
    #[serde(alias = "M60S+VL10")]
    M60SPlusVL10,
    #[serde(alias = "M60S+VL100")]
    M60SPlusVL100,
    #[serde(alias = "M60S+VL20")]
    M60SPlusVL20,
    #[serde(alias = "M60S+VL30")]
    M60SPlusVL30,
    #[serde(alias = "M60S+VL40")]
    M60SPlusVL40,
    #[serde(alias = "M60S+VL50")]
    M60SPlusVL50,
    #[serde(alias = "M60S+VL60")]
    M60SPlusVL60,
    #[serde(alias = "M60S+VL70")]
    M60SPlusVL70,
    #[serde(alias = "M60S+VL80")]
    M60SPlusVL80,
    #[serde(alias = "M60S+VL90")]
    M60SPlusVL90,
    #[serde(alias = "M60S+VLA0")]
    M60SPlusVLA0,
    #[serde(alias = "M60S+VLB0")]
    M60SPlusVLB0,
    #[serde(alias = "M60S+VM20")]
    M60SPlusVM20,
    #[serde(alias = "M60S+VM30")]
    M60SPlusVM30,
    #[serde(alias = "M60S+VM40")]
    M60SPlusVM40,
    #[serde(alias = "M60S+VM50")]
    M60SPlusVM50,
    #[serde(alias = "M60SVK10")]
    M60SVK10,
    #[serde(alias = "M60SVK20")]
    M60SVK20,
    #[serde(alias = "M60SVK30")]
    M60SVK30,
    #[serde(alias = "M60SVK40")]
    M60SVK40,
    #[serde(alias = "M60SVK60")]
    M60SVK60,
    #[serde(alias = "M60SVK70")]
    M60SVK70,
    #[serde(alias = "M60SVK80")]
    M60SVK80,
    #[serde(alias = "M60SVK90")]
    M60SVK90,
    #[serde(alias = "M60SVL10")]
    M60SVL10,
    #[serde(alias = "M60SVL20")]
    M60SVL20,
    #[serde(alias = "M60SVL30")]
    M60SVL30,
    #[serde(alias = "M60SVL40")]
    M60SVL40,
    #[serde(alias = "M60SVL50")]
    M60SVL50,
    #[serde(alias = "M60SVL60")]
    M60SVL60,
    #[serde(alias = "M60SVL70")]
    M60SVL70,
    #[serde(alias = "M60SVL80")]
    M60SVL80,
    #[serde(alias = "M60SVL90")]
    M60SVL90,
    #[serde(alias = "M60SVM20")]
    M60SVM20,
    #[serde(alias = "M60SVM40")]
    M60SVM40,
    #[serde(alias = "M60VK10")]
    M60VK10,
    #[serde(alias = "M60VK20")]
    M60VK20,
    #[serde(alias = "M60VK30")]
    M60VK30,
    #[serde(alias = "M60VK40")]
    M60VK40,
    #[serde(alias = "M60VK6A")]
    M60VK6A,
    #[serde(alias = "M60VL10")]
    M60VL10,
    #[serde(alias = "M60VL20")]
    M60VL20,
    #[serde(alias = "M60VL30")]
    M60VL30,
    #[serde(alias = "M60VL40")]
    M60VL40,
    #[serde(alias = "M60VL50")]
    M60VL50,
    #[serde(alias = "M60VM40")]
    M60VM40,
    #[serde(alias = "M61S+VL20")]
    M61SPlusVL20,
    #[serde(alias = "M61S+VL30")]
    M61SPlusVL30,
    #[serde(alias = "M61SVK20")]
    M61SVK20,
    #[serde(alias = "M61SVK30")]
    M61SVK30,
    #[serde(alias = "M61SVL10")]
    M61SVL10,
    #[serde(alias = "M61SVL20")]
    M61SVL20,
    #[serde(alias = "M61SVL30")]
    M61SVL30,
    #[serde(alias = "M61SVL60")]
    M61SVL60,
    #[serde(alias = "M61SVL70")]
    M61SVL70,
    #[serde(alias = "M61SVL90")]
    M61SVL90,
    #[serde(alias = "M61SVM30")]
    M61SVM30,
    #[serde(alias = "M61VK10")]
    M61VK10,
    #[serde(alias = "M61VK20")]
    M61VK20,
    #[serde(alias = "M61VK30")]
    M61VK30,
    #[serde(alias = "M61VK40")]
    M61VK40,
    #[serde(alias = "M61VK60")]
    M61VK60,
    #[serde(alias = "M61VK70")]
    M61VK70,
    #[serde(alias = "M61VL10")]
    M61VL10,
    #[serde(alias = "M61VL30")]
    M61VL30,
    #[serde(alias = "M61VL40")]
    M61VL40,
    #[serde(alias = "M61VL50")]
    M61VL50,
    #[serde(alias = "M61VL60")]
    M61VL60,
    #[serde(alias = "M62S++VM30")]
    M62SPlusPlusVM30,
    #[serde(alias = "M62S+VK30")]
    M62SPlusVK30,
    #[serde(alias = "M63S++VL10")]
    M63SPlusPlusVL10,
    #[serde(alias = "M63S++VL20")]
    M63SPlusPlusVL20,
    #[serde(alias = "M63S++VL40")]
    M63SPlusPlusVL40,
    #[serde(alias = "M63S++VL50")]
    M63SPlusPlusVL50,
    #[serde(alias = "M63S++VL60")]
    M63SPlusPlusVL60,
    #[serde(alias = "M63S++VL70")]
    M63SPlusPlusVL70,
    #[serde(alias = "M63S++VM10")]
    M63SPlusPlusVM10,
    #[serde(alias = "M63S++VM20")]
    M63SPlusPlusVM20,
    #[serde(alias = "M63S++VM30")]
    M63SPlusPlusVM30,
    #[serde(alias = "M63S+VK30")]
    M63SPlusVK30,
    #[serde(alias = "M63S+VL10")]
    M63SPlusVL10,
    #[serde(alias = "M63S+VL20")]
    M63SPlusVL20,
    #[serde(alias = "M63S+VL30")]
    M63SPlusVL30,
    #[serde(alias = "M63S+VL40")]
    M63SPlusVL40,
    #[serde(alias = "M63S+VL50")]
    M63SPlusVL50,
    #[serde(alias = "M63S+VL60")]
    M63SPlusVL60,
    #[serde(alias = "M63S+VL70")]
    M63SPlusVL70,
    #[serde(alias = "M63S+VL80")]
    M63SPlusVL80,
    #[serde(alias = "M63S+VL90")]
    M63SPlusVL90,
    #[serde(alias = "M63S+VLA0")]
    M63SPlusVLA0,
    #[serde(alias = "M63S+VLC0")]
    M63SPlusVLC0,
    #[serde(alias = "M63S+VLD0")]
    M63SPlusVLD0,
    #[serde(alias = "M63S+VM30")]
    M63SPlusVM30,
    #[serde(alias = "M63S+VM40")]
    M63SPlusVM40,
    #[serde(alias = "M63SVK10")]
    M63SVK10,
    #[serde(alias = "M63SVK20")]
    M63SVK20,
    #[serde(alias = "M63SVK30")]
    M63SVK30,
    #[serde(alias = "M63SVK40")]
    M63SVK40,
    #[serde(alias = "M63SVK50")]
    M63SVK50,
    #[serde(alias = "M63SVK60")]
    M63SVK60,
    #[serde(alias = "M63SVK70")]
    M63SVK70,
    #[serde(alias = "M63SVK80")]
    M63SVK80,
    #[serde(alias = "M63SVK90")]
    M63SVK90,
    #[serde(alias = "M63SVKA0")]
    M63SVKA0,
    #[serde(alias = "M63SVL10")]
    M63SVL10,
    #[serde(alias = "M63SVL20")]
    M63SVL20,
    #[serde(alias = "M63SVL30")]
    M63SVL30,
    #[serde(alias = "M63SVL40")]
    M63SVL40,
    #[serde(alias = "M63SVL50")]
    M63SVL50,
    #[serde(alias = "M63SVL60")]
    M63SVL60,
    #[serde(alias = "M63SVL70")]
    M63SVL70,
    #[serde(alias = "M63SVL80")]
    M63SVL80,
    #[serde(alias = "M63SVL90")]
    M63SVL90,
    #[serde(alias = "M63SVLA0")]
    M63SVLA0,
    #[serde(alias = "M63SVM30")]
    M63SVM30,
    #[serde(alias = "M63VK10")]
    M63VK10,
    #[serde(alias = "M63VK20")]
    M63VK20,
    #[serde(alias = "M63VK30")]
    M63VK30,
    #[serde(alias = "M63VL10")]
    M63VL10,
    #[serde(alias = "M63VL20")]
    M63VL20,
    #[serde(alias = "M63VL30")]
    M63VL30,
    #[serde(alias = "M63VL40")]
    M63VL40,
    #[serde(alias = "M63VL60")]
    M63VL60,
    #[serde(alias = "M63VL70")]
    M63VL70,
    #[serde(alias = "M64S++VM30")]
    M64SPlusPlusVM30,
    #[serde(alias = "M64SVL10")]
    M64SVL10,
    #[serde(alias = "M64SVL20")]
    M64SVL20,
    #[serde(alias = "M64SVL30")]
    M64SVL30,
    #[serde(alias = "M64VL20")]
    M64VL20,
    #[serde(alias = "M64VL30")]
    M64VL30,
    #[serde(alias = "M64VL40")]
    M64VL40,
    #[serde(alias = "M65S+VK30")]
    M65SPlusVK30,
    #[serde(alias = "M65S+VL20")]
    M65SPlusVL20,
    #[serde(alias = "M65SVK20")]
    M65SVK20,
    #[serde(alias = "M65SVL60")]
    M65SVL60,
    #[serde(alias = "M66S++VL20")]
    M66SPlusPlusVL20,
    #[serde(alias = "M66S++VL40")]
    M66SPlusPlusVL40,
    #[serde(alias = "M66S++VL50")]
    M66SPlusPlusVL50,
    #[serde(alias = "M66S++VL60")]
    M66SPlusPlusVL60,
    #[serde(alias = "M66S++VL70")]
    M66SPlusPlusVL70,
    #[serde(alias = "M66S++VL80")]
    M66SPlusPlusVL80,
    #[serde(alias = "M66S++VM30")]
    M66SPlusPlusVM30,
    #[serde(alias = "M66S+VK30")]
    M66SPlusVK30,
    #[serde(alias = "M66S+VL10")]
    M66SPlusVL10,
    #[serde(alias = "M66S+VL20")]
    M66SPlusVL20,
    #[serde(alias = "M66S+VL30")]
    M66SPlusVL30,
    #[serde(alias = "M66S+VL40")]
    M66SPlusVL40,
    #[serde(alias = "M66S+VL50")]
    M66SPlusVL50,
    #[serde(alias = "M66S+VL60")]
    M66SPlusVL60,
    #[serde(alias = "M66S+VL70")]
    M66SPlusVL70,
    #[serde(alias = "M66S+VL80")]
    M66SPlusVL80,
    #[serde(alias = "M66SVK20")]
    M66SVK20,
    #[serde(alias = "M66SVK30")]
    M66SVK30,
    #[serde(alias = "M66SVK40")]
    M66SVK40,
    #[serde(alias = "M66SVK50")]
    M66SVK50,
    #[serde(alias = "M66SVK60")]
    M66SVK60,
    #[serde(alias = "M66SVK70")]
    M66SVK70,
    #[serde(alias = "M66SVK80")]
    M66SVK80,
    #[serde(alias = "M66SVL10")]
    M66SVL10,
    #[serde(alias = "M66SVL20")]
    M66SVL20,
    #[serde(alias = "M66SVL30")]
    M66SVL30,
    #[serde(alias = "M66SVL40")]
    M66SVL40,
    #[serde(alias = "M66SVL50")]
    M66SVL50,
    #[serde(alias = "M66SVL80")]
    M66SVL80,
    #[serde(alias = "M66VK20")]
    M66VK20,
    #[serde(alias = "M66VK30")]
    M66VK30,
    #[serde(alias = "M66VK60")]
    M66VK60,
    #[serde(alias = "M66VL20")]
    M66VL20,
    #[serde(alias = "M66VL30")]
    M66VL30,
    #[serde(alias = "M67SVK30")]
    M67SVK30,
    #[serde(alias = "M69S++VM30")]
    M69SPlusPlusVM30,
    #[serde(alias = "M69VK30")]
    M69VK30,
    #[serde(alias = "M70S+VM30")]
    M70SPlusVM30,
    #[serde(alias = "M70SVM30")]
    M70SVM30,
    #[serde(alias = "M70VL20")]
    M70VL20,
    #[serde(alias = "M70VL30")]
    M70VL30,
    #[serde(alias = "M70VM10")]
    M70VM10,
    #[serde(alias = "M70VM20")]
    M70VM20,
    #[serde(alias = "M70VM30")]
    M70VM30,
    #[serde(alias = "M70VM80")]
    M70VM80,
    #[serde(alias = "M72SVM30")]
    M72SVM30,
    #[serde(alias = "M72VM30")]
    M72VM30,
    #[serde(alias = "M73S+VM40")]
    M73SPlusVM40,
    #[serde(alias = "M73SVM30")]
    M73SVM30,
    #[serde(alias = "M73VL30")]
    M73VL30,
    #[serde(alias = "M73VM20")]
    M73VM20,
    #[serde(alias = "M73VM30")]
    M73VM30,
    #[serde(alias = "M73VM70")]
    M73VM70,
    #[serde(alias = "M76SVM30")]
    M76SVM30,
    #[serde(alias = "M76VL30")]
    M76VL30,
    #[serde(alias = "M76VM30")]
    M76VM30,
    #[serde(alias = "M78SVM30")]
    M78SVM30,
    #[serde(alias = "M79SVM30")]
    M79SVM30,
}
