import {
  useEffect,
  useState,
  type ReactNode,
  type MouseEventHandler,
} from "react";
import { motion, useIsPresent, useReducedMotion } from "motion/react";
import { motionTiming } from "./motion-spec";

export function useViewportWidth() {
  const [width, setWidth] = useState(() => window.innerWidth);
  useEffect(() => {
    let frame = 0;
    const resize = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => setWidth(window.innerWidth));
    };
    window.addEventListener("resize", resize);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
    };
  }, []);
  return width;
}
export function Selection({ active, id }: { active: boolean; id: string }) {
  const reduced = !!useReducedMotion();
  return active ? (
    <motion.span
      className="selection-fill"
      layoutId={id}
      aria-hidden="true"
      transition={motionTiming(reduced, "navigation")}
    />
  ) : null;
}
export function AnimatedPopover({
  children,
  className,
  id,
  role,
  "aria-label": label,
}: {
  children: ReactNode;
  className: string;
  id?: string;
  role?: React.AriaRole;
  "aria-label"?: string;
}) {
  const reduced = !!useReducedMotion(),
    present = useIsPresent();
  const above = /agent-popover|mention-popover/.test(className);
  return (
    <motion.div
      id={id}
      role={role}
      aria-label={label}
      className={className}
      style={{
        transformOrigin: above ? "bottom left" : "top right",
        pointerEvents: present ? "auto" : "none",
      }}
      initial={{
        opacity: 0,
        y: reduced ? 0 : above ? 6 : -6,
        scale: reduced ? 1 : 0.97,
      }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{
        opacity: 0,
        y: reduced ? 0 : above ? 4 : -4,
        scale: reduced ? 1 : 0.98,
      }}
      transition={motionTiming(reduced, "overlay")}
    >
      {children}
    </motion.div>
  );
}
export function AnimatedOverlay({
  children,
  onMouseDown,
}: {
  children: ReactNode;
  onMouseDown: MouseEventHandler<HTMLDivElement>;
}) {
  const reduced = !!useReducedMotion(),
    present = useIsPresent();
  return (
    <motion.div
      className="modal-backdrop"
      onMouseDown={onMouseDown}
      style={{ pointerEvents: present ? "auto" : "none" }}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={motionTiming(reduced, "overlay")}
    >
      {children}
    </motion.div>
  );
}
export function AnimatedCheck({ size = 18 }: { size?: number }) {
  const reduced = !!useReducedMotion();
  return (
    <motion.svg
      className="animated-check"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      initial={{ scale: reduced ? 1 : 0.8 }}
      animate={{ scale: 1 }}
      transition={motionTiming(reduced, "completion")}
    >
      <motion.circle
        cx="12"
        cy="12"
        r="9"
        stroke="currentColor"
        strokeWidth="1.5"
        initial={{ opacity: 0 }}
        animate={{ opacity: 0.65 }}
        transition={motionTiming(reduced, "control")}
      />
      <motion.path
        d="m7.5 12 3 3 6-6"
        stroke="currentColor"
        strokeWidth="1.9"
        strokeLinecap="round"
        strokeLinejoin="round"
        initial={{ pathLength: reduced ? 1 : 0 }}
        animate={{ pathLength: 1 }}
        transition={{
          ...motionTiming(reduced, "completion"),
          delay: reduced ? 0 : 0.06,
        }}
      />
    </motion.svg>
  );
}
