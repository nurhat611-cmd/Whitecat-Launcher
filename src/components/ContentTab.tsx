import { Palette, Puzzle, Sun } from "lucide-react";

const SECTIONS = [
  {
    icon: Palette,
    title: "Ресурспаки",
    text: "Смена графики, текстур и интерфейса. Управление появится во 2-й части.",
  },
  {
    icon: Sun,
    title: "Шейдеры",
    text: "Реалистичное освещение и вода. Управление появится во 2-й части.",
  },
  {
    icon: Puzzle,
    title: "Моды",
    text: "Расширения игрового процесса. Управление появится во 2-й части.",
  },
];

export function ContentTab() {
  return (
    <div>
      <div className="tab-head">
        <h1>Контент</h1>
        <p>Ресурспаки, шейдеры и моды для вашей игры</p>
      </div>
      <div className="cards-3">
        {SECTIONS.map((s) => {
          const Icon = s.icon;
          return (
            <div key={s.title} className="card content-card">
              <span className="content-ic">
                <Icon size={20} />
              </span>
              <h3>{s.title}</h3>
              <p>{s.text}</p>
              <button className="btn" type="button" disabled>
                Управлять · скоро
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
