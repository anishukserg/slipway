# slipway-work

Слой работы [Slipway](https://github.com/anishukserg/slipway): направления, срезы и единицы работы, записанные в реестре на Rust макросами `thrust!`, `slice!` и `work!`.

- **Происхождение работы** — `WorkOrigin`: решение, спецификация, расхождение фактического с заявленным, исследование или уборка замещённого. Работа без основания невыразима; у исследования нет поля, объявляющего приземление кода.
- **Радиус не выше потолка среза** — константная проверка, порождённая сканом: нарушение не компилируется.
- **Состояния у записи нет** — `WorkState` вычисляется свёрткой журнала при сборке реестра ([slipway-journal](https://github.com/anishukserg/slipway/tree/master/crates/slipway-journal)).

Поля появляются вместе с механизмом, который их читает: схема намеренно минимальна.

План самого Slipway, записанный этим слоем, — [doc](https://github.com/anishukserg/slipway/tree/master/doc).

Лицензия — [Apache-2.0](https://github.com/anishukserg/slipway/blob/master/LICENSE).
