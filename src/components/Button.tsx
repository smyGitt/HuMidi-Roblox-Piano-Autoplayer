import type { AnchorHTMLAttributes, ButtonHTMLAttributes } from "react";

export type ButtonVariant = "default" | "accent" | "icon" | "tab" | "nav" | "item" | "bare";
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonStyleProps {
  variant?: ButtonVariant;
  size?: ButtonSize;
  active?: boolean;
  danger?: boolean;
  subtle?: boolean;
  outlined?: boolean;
}

export function buttonClassName(
  { variant = "default", size, active, danger, subtle, outlined }: ButtonStyleProps,
  className?: string,
) {
  const resolvedSize = size ?? (variant === "icon" ? "sm" : undefined);
  return [
    "btn",
    `btn--${variant}`,
    resolvedSize && `btn--${resolvedSize}`,
    active && "btn--active",
    danger && "btn--danger",
    subtle && "btn--subtle",
    outlined && "btn--outlined",
    className,
  ]
    .filter(Boolean)
    .join(" ");
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & ButtonStyleProps;

export function Button({ variant, size, active, danger, subtle, outlined, className, type = "button", ...rest }: ButtonProps) {
  return (
    <button
      type={type}
      className={buttonClassName({ variant, size, active, danger, subtle, outlined }, className)}
      {...rest}
    />
  );
}

type LinkProps = AnchorHTMLAttributes<HTMLAnchorElement> & ButtonStyleProps;

export function Link({ variant, size, active, danger, subtle, outlined, className, ...rest }: LinkProps) {
  return (
    <a className={buttonClassName({ variant, size, active, danger, subtle, outlined }, className)} {...rest} />
  );
}
