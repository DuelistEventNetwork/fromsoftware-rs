#[repr(C)]
pub struct FaceData {
    vftable: usize,
    pub face_data_buffer: FaceDataBuffer,
    unk128: usize,
    unk130: [f32; 7],
    unk14c: [u8; 0x24],
}

#[repr(C)]
pub struct FaceDataBuffer {
    /// `"FACE"`.
    pub magic: [u8; 4],
    /// `4` for 1.16.2's `FaceDataBuffer` layout.
    pub version: u32,
    pub buffer_size: u32,
    /// The character creator's settings.
    pub face_model: FaceModel,
    unk10c: [u8; 12],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FaceColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FaceHairColor {
    pub color: FaceColor,
    pub shininess: u8,
    pub root_black: u8,
    pub white_density: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FaceEye {
    pub iris_color: FaceColor,
    pub iris_scale: u8,
    pub cataract: u8,
    pub cataract_color: FaceColor,
    pub sclera_color: FaceColor,
    pub iris_distance: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceModel {
    /// The base template, packed as decimal digits `XYZ` where:
    ///
    /// - `X` (hundreds) is a musculature
    /// - `Y` (tens) is a bone structure
    /// - `Z` (units) is an age
    ///
    /// So `120` is musculature `1`, bone structure `2`, age `0` (Young).
    pub face_model_id: u32,
    pub hair_model_id: u32,
    pub eye_model_id: u32,
    pub eyebrow_model_id: u32,
    pub beard_model_id: u32,
    pub accessories_model_id: u32,
    pub decal_model_id: u32,
    pub eyelash_model_id: u32,
    pub apparent_age: u8,
    pub facial_aesthetic: u8,
    pub form_emphasis: u8,
    pub numen: u8,
    pub brow_ridge_height: u8,
    pub inner_brow_ridge: u8,
    pub outer_brow_ridge: u8,
    pub cheekbone_height: u8,
    pub cheekbone_depth: u8,
    pub cheekbone_width: u8,
    pub cheekbone_protrusion: u8,
    pub cheeks: u8,
    pub chin_tip_position: u8,
    pub chin_length: u8,
    pub chin_protrusion: u8,
    pub chin_depth: u8,
    pub chin_size: u8,
    pub chin_height: u8,
    pub chin_width: u8,
    pub eye_position: u8,
    pub eye_size: u8,
    pub eye_slant: u8,
    pub eye_spacing: u8,
    pub nose_size: u8,
    pub nose_forehead_ratio: u8,
    pub unused39: u8,
    pub face_protrusion: u8,
    pub vertical_face_ratio: u8,
    pub facial_feature_slant: u8,
    pub horizontal_face_ratio: u8,
    pub unused3e: u8,
    pub forehead_depth: u8,
    pub forehead_protrusion: u8,
    pub unused41: u8,
    pub jaw_protrusion: u8,
    pub jaw_width: u8,
    pub lower_jaw: u8,
    pub jaw_contour: u8,
    pub lip_shape: u8,
    pub lip_size: u8,
    pub lip_fullness: u8,
    pub lip_thickness_unk: u8,
    pub lip_protrusion: u8,
    pub lip_thickness: u8,
    pub mouth_protrusion: u8,
    pub mouth_slant: u8,
    pub mouth_occlusion: u8,
    pub mouth_position: u8,
    pub mouth_width: u8,
    pub mouth_chin_distance: u8,
    pub nose_ridge_depth: u8,
    pub nose_ridge_length: u8,
    pub nose_position: u8,
    pub nose_tip_height: u8,
    pub nostril_slant: u8,
    pub nostril_size: u8,
    pub nostril_width: u8,
    pub nose_protrusion: u8,
    pub nose_bridge_height: u8,
    pub nose_bridge_protrusion1: u8,
    pub nose_bridge_protrusion2: u8,
    pub nose_bridge_width: u8,
    pub nose_height: u8,
    pub nose_slant: u8,
    pub unused60: u8,
    pub face_tex_data: [u8; 0x24],
    pub face_geo_asym_data: [u8; 26],
    pub unused9f: u8,
    pub body_scale_head: u8,
    pub body_scale_breast: u8,
    pub body_scale_abdomen: u8,
    pub body_scale_arm_right: u8,
    pub body_scale_leg_right: u8,
    pub body_scale_arm_left: u8,
    pub body_scale_leg_left: u8,

    pub skin_color: FaceColor,
    pub skin_gloss: u8,
    pub skin_pores: u8,
    pub face_beard: u8,
    /// Dark circles transparency.
    pub face_around_eye: u8,
    pub face_around_eye_color: FaceColor,
    /// Cheek blush transparency.
    pub face_cheek: u8,
    pub face_cheek_color: FaceColor,
    /// Eyeliner transparency.
    pub face_eye_line: u8,
    pub face_eye_line_color: FaceColor,
    /// Lower eyeshadow transparency.
    pub face_eye_shadow_down: u8,
    pub face_eye_shadow_down_color: FaceColor,
    /// Upper eyeshadow transparency.
    pub face_eye_shadow_up: u8,
    pub face_eye_shadow_up_color: FaceColor,
    /// Lipstick transparency.
    pub face_lip: u8,
    pub face_lip_color: FaceColor,
    pub decal_pos_x: u8,
    pub decal_pos_y: u8,
    pub decal_angle: u8,
    pub decal_scale: u8,
    pub decal_color: FaceColor,
    /// Unused field, but present in the `FaceParam` struct.
    pub decal_gloss: u8,
    pub decal_mirror: MirrorState,
    /// Body hair transparency.
    pub body_hair: u8,
    pub body_hair_color: FaceColor,

    pub eye_right: FaceEye,
    pub eye_left: FaceEye,

    pub hair: FaceHairColor,
    pub beard: FaceHairColor,
    pub eyebrow: FaceHairColor,
    pub eyelash_color: FaceColor,
    pub accessories_color: FaceColor,
    /// Transparency of the frenzied flame body decal.
    pub frenzied_flame: u8,
    pub unused103: [u8; 5],
}

impl FaceModel {
    /// The units digit of [`face_model_id`](Self::face_model_id).
    pub fn template_age(&self) -> u32 {
        self.face_model_id % 10
    }

    /// The tens digit of [`face_model_id`](Self::face_model_id).
    pub fn template_bone_structure(&self) -> u32 {
        (self.face_model_id / 10) % 10
    }

    /// The hundreds digit of [`face_model_id`](Self::face_model_id).
    pub fn template_musculature(&self) -> u32 {
        (self.face_model_id / 100) % 10
    }

    /// Packs the three template digits back into
    /// [`face_model_id`](Self::face_model_id), modulo 10 to ensure they are single digits.
    pub fn set_face_template(&mut self, age: u32, bone_structure: u32, musculature: u32) {
        self.face_model_id = (age % 10) + (bone_structure % 10) * 10 + (musculature % 10) * 100;
    }

    pub const fn preset(body_type: BodyType) -> Self {
        match body_type {
            BodyType::A => Self::PRESET_A,
            BodyType::B => Self::PRESET_B,
        }
    }

    /// `FaceParam` row `1000`, "26: Warrior Male".
    pub const PRESET_A: Self = Self {
        // model ids
        face_model_id: 0,
        hair_model_id: 9,
        eye_model_id: 0,
        eyebrow_model_id: 3,
        beard_model_id: 1,
        accessories_model_id: 0,
        decal_model_id: 0,
        eyelash_model_id: 2,

        apparent_age: 205,
        facial_aesthetic: 108,
        form_emphasis: 0,
        numen: 0,
        brow_ridge_height: 118,
        inner_brow_ridge: 168,
        outer_brow_ridge: 108,
        cheekbone_height: 88,
        cheekbone_depth: 188,
        cheekbone_width: 98,
        cheekbone_protrusion: 148,
        cheeks: 78,
        chin_tip_position: 168,
        chin_length: 98,
        chin_protrusion: 68,
        chin_depth: 128,
        chin_size: 138,
        chin_height: 108,
        chin_width: 108,
        eye_position: 138,
        eye_size: 138,
        eye_slant: 198,
        eye_spacing: 58,
        nose_size: 128,
        nose_forehead_ratio: 178,
        unused39: 0,
        face_protrusion: 128,
        vertical_face_ratio: 78,
        facial_feature_slant: 68,
        horizontal_face_ratio: 115,
        unused3e: 0,
        forehead_depth: 178,
        forehead_protrusion: 128,
        unused41: 0,
        jaw_protrusion: 118,
        jaw_width: 108,
        lower_jaw: 98,
        jaw_contour: 118,
        lip_shape: 128,
        lip_size: 168,
        lip_fullness: 98,
        lip_thickness_unk: 140,
        lip_protrusion: 198,
        lip_thickness: 138,
        mouth_protrusion: 128,
        mouth_slant: 168,
        mouth_occlusion: 128,
        mouth_position: 118,
        mouth_width: 105,
        mouth_chin_distance: 208,
        nose_ridge_depth: 138,
        nose_ridge_length: 128,
        nose_position: 128,
        nose_tip_height: 135,
        nostril_slant: 198,
        nostril_size: 98,
        nostril_width: 198,
        nose_protrusion: 125,
        nose_bridge_height: 38,
        nose_bridge_protrusion1: 188,
        nose_bridge_protrusion2: 85,
        nose_bridge_width: 138,
        nose_height: 158,
        nose_slant: 70,
        unused60: 128,
        unused9f: 0,
        body_scale_head: 128,
        body_scale_breast: 128,
        body_scale_abdomen: 128,
        body_scale_arm_right: 128,
        body_scale_leg_right: 128,
        body_scale_arm_left: 128,
        body_scale_leg_left: 128,
        skin_gloss: 160,
        skin_pores: 255,
        face_beard: 215,
        face_around_eye: 80,
        face_cheek: 0,
        face_eye_line: 0,
        face_eye_shadow_down: 100,
        face_eye_shadow_up: 30,
        face_lip: 20,
        decal_pos_x: 128,
        decal_pos_y: 210,
        decal_angle: 128,
        decal_scale: 128,
        decal_gloss: 128,
        body_hair: 0,
        frenzied_flame: 0,

        skin_color: FaceColor {
            red: 143,
            green: 103,
            blue: 79,
        },
        face_around_eye_color: FaceColor {
            red: 20,
            green: 30,
            blue: 25,
        },
        face_cheek_color: FaceColor {
            red: 255,
            green: 65,
            blue: 65,
        },
        face_eye_line_color: FaceColor {
            red: 0,
            green: 0,
            blue: 0,
        },
        face_eye_shadow_down_color: FaceColor {
            red: 50,
            green: 25,
            blue: 0,
        },
        face_eye_shadow_up_color: FaceColor {
            red: 40,
            green: 20,
            blue: 30,
        },
        face_lip_color: FaceColor {
            red: 255,
            green: 87,
            blue: 87,
        },
        decal_color: FaceColor {
            red: 71,
            green: 37,
            blue: 24,
        },
        body_hair_color: FaceColor {
            red: 70,
            green: 48,
            blue: 29,
        },
        eyelash_color: FaceColor {
            red: 0,
            green: 0,
            blue: 0,
        },
        accessories_color: FaceColor {
            red: 60,
            green: 60,
            blue: 60,
        },

        eye_right: FaceEye {
            iris_color: FaceColor {
                red: 26,
                green: 15,
                blue: 5,
            },
            iris_scale: 200,
            cataract: 0,
            cataract_color: FaceColor {
                red: 100,
                green: 100,
                blue: 100,
            },
            sclera_color: FaceColor {
                red: 255,
                green: 255,
                blue: 255,
            },
            iris_distance: 138,
        },
        eye_left: FaceEye {
            iris_color: FaceColor {
                red: 26,
                green: 15,
                blue: 5,
            },
            iris_scale: 200,
            cataract: 0,
            cataract_color: FaceColor {
                red: 100,
                green: 100,
                blue: 100,
            },
            sclera_color: FaceColor {
                red: 255,
                green: 255,
                blue: 255,
            },
            iris_distance: 138,
        },

        hair: FaceHairColor {
            color: FaceColor {
                red: 70,
                green: 48,
                blue: 29,
            },
            shininess: 78,
            root_black: 128,
            white_density: 80,
        },
        beard: FaceHairColor {
            color: FaceColor {
                red: 70,
                green: 48,
                blue: 29,
            },
            shininess: 78,
            root_black: 128,
            white_density: 80,
        },
        eyebrow: FaceHairColor {
            color: FaceColor {
                red: 70,
                green: 48,
                blue: 29,
            },
            shininess: 78,
            root_black: 128,
            white_density: 80,
        },

        decal_mirror: MirrorState::Off,
        face_tex_data: [
            0, 0, 0, 0, 128, 128, 128, 128, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 128, 128, 128, 0,
            0, 0, 0, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
        ],
        face_geo_asym_data: [
            128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
            128, 128, 128, 128, 128, 128, 128, 128, 128,
        ],
        unused103: [0, 0, 0, 0, 0],
    };

    /// `FaceParam` row `2000`, "36: Warrior Female".
    pub const PRESET_B: Self = Self {
        face_model_id: 10,
        hair_model_id: 105,
        eye_model_id: 0,
        eyebrow_model_id: 15,
        beard_model_id: 0,
        accessories_model_id: 0,
        decal_model_id: 0,
        eyelash_model_id: 2,

        apparent_age: 75,
        facial_aesthetic: 165,
        form_emphasis: 0,
        numen: 0,
        brow_ridge_height: 120,
        inner_brow_ridge: 115,
        outer_brow_ridge: 178,
        cheekbone_height: 140,
        cheekbone_depth: 105,
        cheekbone_width: 128,
        cheekbone_protrusion: 108,
        cheeks: 110,
        chin_tip_position: 128,
        chin_length: 128,
        chin_protrusion: 128,
        chin_depth: 128,
        chin_size: 128,
        chin_height: 128,
        chin_width: 128,
        eye_position: 188,
        eye_size: 118,
        eye_slant: 228,
        eye_spacing: 88,
        nose_size: 128,
        nose_forehead_ratio: 178,
        unused39: 0,
        face_protrusion: 128,
        vertical_face_ratio: 78,
        facial_feature_slant: 68,
        horizontal_face_ratio: 92,
        unused3e: 0,
        forehead_depth: 38,
        forehead_protrusion: 120,
        unused41: 0,
        jaw_protrusion: 128,
        jaw_width: 128,
        lower_jaw: 120,
        jaw_contour: 128,
        lip_shape: 125,
        lip_size: 98,
        lip_fullness: 95,
        lip_thickness_unk: 170,
        lip_protrusion: 208,
        lip_thickness: 68,
        mouth_protrusion: 158,
        mouth_slant: 180,
        mouth_occlusion: 128,
        mouth_position: 205,
        mouth_width: 125,
        mouth_chin_distance: 175,
        nose_ridge_depth: 158,
        nose_ridge_length: 128,
        nose_position: 215,
        nose_tip_height: 125,
        nostril_slant: 128,
        nostril_size: 60,
        nostril_width: 178,
        nose_protrusion: 130,
        nose_bridge_height: 158,
        nose_bridge_protrusion1: 91,
        nose_bridge_protrusion2: 140,
        nose_bridge_width: 128,
        nose_height: 158,
        nose_slant: 160,
        unused60: 128,
        unused9f: 0,
        body_scale_head: 128,
        body_scale_breast: 128,
        body_scale_abdomen: 128,
        body_scale_arm_right: 128,
        body_scale_leg_right: 128,
        body_scale_arm_left: 128,
        body_scale_leg_left: 128,
        skin_gloss: 160,
        skin_pores: 255,
        face_beard: 0,
        face_around_eye: 50,
        face_cheek: 0,
        face_eye_line: 30,
        face_eye_shadow_down: 65,
        face_eye_shadow_up: 40,
        face_lip: 0,
        decal_pos_x: 128,
        decal_pos_y: 210,
        decal_angle: 128,
        decal_scale: 128,
        decal_gloss: 128,
        body_hair: 0,
        frenzied_flame: 0,

        skin_color: FaceColor {
            red: 171,
            green: 125,
            blue: 99,
        },
        face_around_eye_color: FaceColor {
            red: 20,
            green: 30,
            blue: 25,
        },
        face_cheek_color: FaceColor {
            red: 255,
            green: 65,
            blue: 65,
        },
        face_eye_line_color: FaceColor {
            red: 0,
            green: 0,
            blue: 0,
        },
        face_eye_shadow_down_color: FaceColor {
            red: 15,
            green: 9,
            blue: 3,
        },
        face_eye_shadow_up_color: FaceColor {
            red: 40,
            green: 20,
            blue: 30,
        },
        face_lip_color: FaceColor {
            red: 255,
            green: 87,
            blue: 87,
        },
        decal_color: FaceColor {
            red: 71,
            green: 37,
            blue: 24,
        },
        body_hair_color: FaceColor {
            red: 70,
            green: 48,
            blue: 29,
        },
        eyelash_color: FaceColor {
            red: 0,
            green: 0,
            blue: 0,
        },
        accessories_color: FaceColor {
            red: 60,
            green: 60,
            blue: 60,
        },

        eye_right: FaceEye {
            iris_color: FaceColor {
                red: 78,
                green: 103,
                blue: 44,
            },
            iris_scale: 220,
            cataract: 0,
            cataract_color: FaceColor {
                red: 100,
                green: 100,
                blue: 100,
            },
            sclera_color: FaceColor {
                red: 255,
                green: 255,
                blue: 255,
            },
            iris_distance: 128,
        },
        eye_left: FaceEye {
            iris_color: FaceColor {
                red: 78,
                green: 103,
                blue: 44,
            },
            iris_scale: 220,
            cataract: 0,
            cataract_color: FaceColor {
                red: 100,
                green: 100,
                blue: 100,
            },
            sclera_color: FaceColor {
                red: 255,
                green: 255,
                blue: 255,
            },
            iris_distance: 128,
        },

        hair: FaceHairColor {
            color: FaceColor {
                red: 70,
                green: 48,
                blue: 29,
            },
            shininess: 148,
            root_black: 128,
            white_density: 30,
        },
        beard: FaceHairColor {
            color: FaceColor {
                red: 70,
                green: 48,
                blue: 29,
            },
            shininess: 148,
            root_black: 128,
            white_density: 30,
        },
        eyebrow: FaceHairColor {
            color: FaceColor {
                red: 70,
                green: 48,
                blue: 29,
            },
            shininess: 148,
            root_black: 128,
            white_density: 30,
        },

        decal_mirror: MirrorState::Off,
        face_tex_data: [
            0, 0, 0, 0, 128, 128, 128, 128, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 128, 128, 128, 0,
            0, 0, 0, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
        ],
        face_geo_asym_data: [
            128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
            128, 128, 128, 128, 128, 128, 128, 128, 128,
        ],
        unused103: [0, 0, 0, 0, 0],
    };
}

impl Default for FaceModel {
    fn default() -> Self {
        Self::PRESET_A
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MirrorState {
    On = 0,
    Off = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyType {
    /// `FaceParam` row `1000` ("26: Warrior Male").
    #[default]
    A = 0,
    /// `FaceParam` row `2000` ("36: Warrior Female").
    B = 1,
}

impl BodyType {
    /// The `FaceParam` row this body type's default face comes from.
    pub const fn face_param_id(self) -> u32 {
        match self {
            Self::A => 1000,
            Self::B => 2000,
        }
    }
}

impl FaceModel {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    pub fn from_face_param(param: &crate::param::FACE_PARAM_ST) -> Self {
        let mut face = Self {
            face_model_id: param.face_parts_id() as u32,
            hair_model_id: param.hair_parts_id() as u32,
            eye_model_id: param.eye_parts_id() as u32,
            eyebrow_model_id: param.eyebrow_parts_id() as u32,
            beard_model_id: param.beard_parts_id() as u32,
            accessories_model_id: param.accessories_parts_id() as u32,
            decal_model_id: param.decal_parts_id() as u32,
            eyelash_model_id: param.eyelash_parts_id() as u32,

            frenzied_flame: param.burn_scar(),

            body_scale_head: param.chr_body_scale_head(),
            body_scale_breast: param.chr_body_scale_breast(),
            body_scale_abdomen: param.chr_body_scale_abdomen(),
            body_scale_arm_right: param.chr_body_scale_r_arm(),
            body_scale_arm_left: param.chr_body_scale_l_arm(),
            body_scale_leg_right: param.chr_body_scale_r_leg(),
            body_scale_leg_left: param.chr_body_scale_l_leg(),

            skin_color: FaceColor {
                red: param.skin_color_r(),
                green: param.skin_color_g(),
                blue: param.skin_color_b(),
            },

            hair: FaceHairColor {
                color: FaceColor {
                    red: param.hair_color_r(),
                    green: param.hair_color_g(),
                    blue: param.hair_color_b(),
                },
                ..Default::default()
            },

            ..Default::default()
        };

        face.copy_data_arrays_from(param);
        face
    }

    fn copy_data_arrays_from(&mut self, param: &crate::param::FACE_PARAM_ST) {
        self.slider_run_mut().copy_from_slice(&[
            param.face_geo_data00(),
            param.face_geo_data01(),
            param.face_geo_data02(),
            param.face_geo_data03(),
            param.face_geo_data04(),
            param.face_geo_data05(),
            param.face_geo_data06(),
            param.face_geo_data07(),
            param.face_geo_data08(),
            param.face_geo_data09(),
            param.face_geo_data10(),
            param.face_geo_data11(),
            param.face_geo_data12(),
            param.face_geo_data13(),
            param.face_geo_data14(),
            param.face_geo_data15(),
            param.face_geo_data16(),
            param.face_geo_data17(),
            param.face_geo_data18(),
            param.face_geo_data19(),
            param.face_geo_data20(),
            param.face_geo_data21(),
            param.face_geo_data22(),
            param.face_geo_data23(),
            param.face_geo_data24(),
            param.face_geo_data25(),
            param.face_geo_data26(),
            param.face_geo_data27(),
            param.face_geo_data28(),
            param.face_geo_data29(),
            param.face_geo_data30(),
            param.face_geo_data31(),
            param.face_geo_data32(),
            param.face_geo_data33(),
            param.face_geo_data34(),
            param.face_geo_data35(),
            param.face_geo_data36(),
            param.face_geo_data37(),
            param.face_geo_data38(),
            param.face_geo_data39(),
            param.face_geo_data40(),
            param.face_geo_data41(),
            param.face_geo_data42(),
            param.face_geo_data43(),
            param.face_geo_data44(),
            param.face_geo_data45(),
            param.face_geo_data46(),
            param.face_geo_data47(),
            param.face_geo_data48(),
            param.face_geo_data49(),
            param.face_geo_data50(),
            param.face_geo_data51(),
            param.face_geo_data52(),
            param.face_geo_data53(),
            param.face_geo_data54(),
            param.face_geo_data55(),
            param.face_geo_data56(),
            param.face_geo_data57(),
            param.face_geo_data58(),
            param.face_geo_data59(),
            param.face_geo_data60(),
        ]);
        self.face_tex_data.copy_from_slice(&[
            param.face_tex_data00(),
            param.face_tex_data01(),
            param.face_tex_data02(),
            param.face_tex_data03(),
            param.face_tex_data04(),
            param.face_tex_data05(),
            param.face_tex_data06(),
            param.face_tex_data07(),
            param.face_tex_data08(),
            param.face_tex_data09(),
            param.face_tex_data10(),
            param.face_tex_data11(),
            param.face_tex_data12(),
            param.face_tex_data13(),
            param.face_tex_data14(),
            param.face_tex_data15(),
            param.face_tex_data16(),
            param.face_tex_data17(),
            param.face_tex_data18(),
            param.face_tex_data19(),
            param.face_tex_data20(),
            param.face_tex_data21(),
            param.face_tex_data22(),
            param.face_tex_data23(),
            param.face_tex_data24(),
            param.face_tex_data25(),
            param.face_tex_data26(),
            param.face_tex_data27(),
            param.face_tex_data28(),
            param.face_tex_data29(),
            param.face_tex_data30(),
            param.face_tex_data31(),
            param.face_tex_data32(),
            param.face_tex_data33(),
            param.face_tex_data34(),
            param.face_tex_data35(),
        ]);
        self.face_geo_asym_data.copy_from_slice(&[
            param.face_geo_asym_data00(),
            param.face_geo_asym_data01(),
            param.face_geo_asym_data02(),
            param.face_geo_asym_data03(),
            param.face_geo_asym_data04(),
            param.face_geo_asym_data05(),
            param.face_geo_asym_data06(),
            param.face_geo_asym_data07(),
            param.face_geo_asym_data08(),
            param.face_geo_asym_data09(),
            param.face_geo_asym_data10(),
            param.face_geo_asym_data11(),
            param.face_geo_asym_data12(),
            param.face_geo_asym_data13(),
            param.face_geo_asym_data14(),
            param.face_geo_asym_data15(),
            param.face_geo_asym_data16(),
            param.face_geo_asym_data17(),
            param.face_geo_asym_data18(),
            param.face_geo_asym_data19(),
            param.face_geo_asym_data20(),
            param.face_geo_asym_data21(),
            param.face_geo_asym_data22(),
            param.face_geo_asym_data23(),
            param.face_geo_asym_data24(),
            param.face_geo_asym_data25(),
        ]);
    }

    fn slider_run_mut(&mut self) -> &mut [u8] {
        const FACE_GEO_DATA_LEN: usize = std::mem::offset_of!(FaceModel, face_tex_data)
            - std::mem::offset_of!(FaceModel, brow_ridge_height);
        const _: () = assert!(FACE_GEO_DATA_LEN == 61);

        // Safety: the run is `FACE_GEO_DATA_LEN` consecutive `u8` fields
        // starting at `brow_ridge_height`.
        unsafe {
            std::slice::from_raw_parts_mut(
                &mut self.brow_ridge_height as *mut u8,
                FACE_GEO_DATA_LEN,
            )
        }
    }
}
