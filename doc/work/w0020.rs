use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(20,
    title: NonEmptyStr::new("Блокировка коммита без номера процесса ждала до тайм-аута"),
    slice: crate::slice::s0006,
    origin: WorkOrigin::Divergence {
        specification: crate::rfc::r0002,
        violated: NonEmptyStr::new(
            "Параллельный коммит ждёт блокировку, а блокировку умершего процесса снимает ожидающий (решения 8 и 14)."
        ),
    },
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Сценарий, падавший до починки, проходит: пустой файл блокировки старше порога — след flock прежнего скрипта или сбой между созданием файла и записью номера — снимается; свежий файл без номера уважается; отказ по тайм-ауту называет держателя."
    ),
);
