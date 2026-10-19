import { motion } from "framer-motion";
import { Boxes, Home, Settings, type LucideIcon } from "lucide-react";
import type { Tab } from "../types";
import { CatLogo } from "./CatLogo";

const ITEMS: { id: Tab; label: string; icon: LucideIcon }[] = [
  { id: "home", label: "Главная", icon: Home },
  { id: "content", label: "Контент", icon: Boxes },
  { id: "settings", label: "Настройки", icon: Settings },
];

export function Sidebar({ tab, onTab }: { tab: Tab; onTab: (t: Tab) => void }) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <CatLogo size={34} />
        <div className="brand-name">
          My<span>Launcher</span>
        </div>
      </div>
      <nav className="nav">
        {ITEMS.map((item) => {
          const Icon = item.icon;
          const active = tab === item.id;
          return (
            <button
              key={item.id}
              className={`nav-btn${active ? " active" : ""}`}
              onClick={() => onTab(item.id)}
            >
              {active && (
                <motion.span
                  layoutId="nav-pill"
                  className="nav-pill"
                  transition={{ type: "spring", stiffness: 420, damping: 34 }}
                />
              )}
              <span className="nav-ic">
                <Icon size={18} strokeWidth={2.2} />
              </span>
              <span className="nav-tx">{item.label}</span>
            </button>
          );
        })}
      </nav>
      <div className="sidebar-foot">v0.1.0 · часть 1/4</div>
    </aside>
  );
}
