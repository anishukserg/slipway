use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(25,
    title: NonEmptyStr::new("Порождённый код и макросы регистрации под строгим профилем lints"),
    slice: crate::slice::s0008,
    origin: WorkOrigin::Decision(crate::adr::a0016),
    taxon: taxon!(Subsystem, Scan),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Реестры doc/ и examples/demo-doc объявляют профиль решения 16 на уровне forbid и проходят калитку; до починки тот же профиль давал отказ clippy на порождённом коде; каждый публичный элемент порождённого кода и макросов регистрации документирован."
    ),
);
