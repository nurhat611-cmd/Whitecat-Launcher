import { motion } from "framer-motion";
import { CatLogo } from "./CatLogo";

const EASE = [0.22, 1, 0.36, 1] as const;

export function Splash() {
  return (
    <motion.div
      className="splash"
      exit={{ opacity: 0, scale: 1.06, filter: "blur(8px)" }}
      transition={{ duration: 0.55, ease: "easeInOut" }}
    >
      <motion.div
        className="splash-logo"
        initial={{ opacity: 0, y: 20, scale: 0.82 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ duration: 0.8, ease: EASE }}
      >
        <CatLogo size={112} />
      </motion.div>
      <motion.h1
        className="splash-name"
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, delay: 0.15, ease: EASE }}
      >
        My<span>Launcher</span>
      </motion.h1>
      <motion.div
        className="splash-bar"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.35, duration: 0.4 }}
      >
        <i />
      </motion.div>
    </motion.div>
  );
}
