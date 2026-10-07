import { Check, Search, X } from "lucide-react";
import { forwardRef, useEffect, useRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { api } from "../lib/api";
import { t } from "../lib/i18n";

type ButtonVariant = "primary" | "secondary" | "danger" | "quiet";

let bodyScrollLockCount = 0;
let bodyOverflowBeforeLock = "";
let inertRootLockCount = 0;

function acquireDialogBoundary() {
  const background = document.getElementById("root");
  if (bodyScrollLockCount === 0) {
    bodyOverflowBeforeLock = document.body.style.overflow;
    document.body.style.overflow = "hidden";
  }
  bodyScrollLockCount += 1;
  inertRootLockCount += 1;
  if (background) background.inert = true;
  return () => {
    bodyScrollLockCount = Math.max(0, bodyScrollLockCount - 1);
    inertRootLockCount = Math.max(0, inertRootLockCount - 1);
    if (bodyScrollLockCount === 0) {
      document.body.style.overflow = bodyOverflowBeforeLock;
      bodyOverflowBeforeLock = "";
    }
    if (background && inertRootLockCount === 0) background.inert = false;
  };
}

export function Button({
  variant = "primary",
  className = "",
  onClick,
  disabled,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  const inert = type !== "submit" && !onClick;
  return (
    <button
      type={type}
      className={`button button--${variant} ${className}`}
      onClick={onClick}
      disabled={disabled || inert}
      {...props}
    />
  );
}

export function StatusBadge({
  tone = "neutral",
  children,
}: {
  tone?: "neutral" | "ok" | "warning" | "danger";
  children: ReactNode;
}) {
  return (
    <span className={`status status--${tone}`}>
      <span className="status__dot" aria-hidden="true" />
      {children}
    </span>
  );
}

export function PageHeader({
  eyebrow,
  title,
  subtitle,
  actions,
}: {
  eyebrow?: string;
  title: string;
  subtitle: string;
  actions?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div className="page-header__copy">
        {eyebrow && <p className="eyebrow">{eyebrow}</p>}
        <h1 tabIndex={-1}>{title}</h1>
        <p>{subtitle}</p>
      </div>
      {actions && <div className="page-actions">{actions}</div>}
    </header>
  );
}

export const SearchField = forwardRef<HTMLInputElement, { value: string; onChange: (value: string) => void }>(
  function SearchField({ value, onChange }, forwardedRef) {
    return (
      <div className="search">
        <Search aria-hidden="true" size={17} />
        <label className="sr-only" htmlFor="capability-search">{t("common.search")}</label>
        <input
          ref={forwardedRef}
          id="capability-search"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={t("common.search")}
        />
        {value && (
          <button type="button" onClick={() => onChange("")} aria-label={t("common.clearSearch")}>
            <X size={15} />
          </button>
        )}
      </div>
    );
  },
);

export function Dialog({
  open,
  title,
  children,
  onClose,
  actions,
  wide = false,
  dismissible = true,
}: {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: () => void;
  actions: ReactNode;
  wide?: boolean;
  dismissible?: boolean;
}) {
  const closeRef = useRef<HTMLButtonElement>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const releaseBoundary = acquireDialogBoundary();
    const frame = closeRef.current?.closest<HTMLElement>("[role='dialog']");
    const focusable = () => Array.from(frame?.querySelectorAll<HTMLElement>("button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])") ?? []);
    (frame?.querySelector<HTMLElement>("[data-autofocus]") ?? closeRef.current)?.focus();
    void api.debugEvent("dialog_open", title);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void api.debugEvent("dialog_escape", title);
        onCloseRef.current();
      }
      if (event.key === "Tab") {
        const nodes = focusable();
        if (!nodes.length) return;
        const first = nodes[0];
        const last = nodes[nodes.length - 1];
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      releaseBoundary();
      previous?.focus();
    };
  }, [open, title]);

  if (!open) return null;
  return createPortal(
    <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className={`dialog ${wide ? "dialog--wide" : ""}`} role="dialog" aria-modal="true" aria-labelledby="dialog-title">
        <header>
          <h2 id="dialog-title">{title}</h2>
          <button ref={closeRef} type="button" className="icon-button" disabled={!dismissible} onClick={onClose} aria-label={t("common.close")}>
            <X size={19} />
          </button>
        </header>
        <div className="dialog-body" onFocusCapture={(event) => {
          if (event.target instanceof HTMLElement) event.target.scrollIntoView?.({ block: "nearest" });
        }}>{children}</div>
        <footer>{actions}</footer>
      </section>
    </div>,
    document.body,
  );
}

export function Toast({ message, tone = "ok" }: { message: string; tone?: "ok" | "danger" }) {
  return (
    <div className={`toast toast--${tone}`} role={tone === "danger" ? "alert" : "status"}>
      <Check size={17} aria-hidden="true" />
      {message}
    </div>
  );
}
