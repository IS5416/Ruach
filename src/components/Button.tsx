import type { ButtonHTMLAttributes, ReactNode } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "ghost" | "primary";
  active?: boolean;
  children: ReactNode;
}

/** Minimal button honoring the quiet design language. */
export function Button({ variant = "ghost", active = false, children, ...rest }: ButtonProps) {
  return (
    <button
      className={`btn btn--${variant}${active ? " btn--active" : ""}`}
      {...rest}
    >
      {children}
    </button>
  );
}
