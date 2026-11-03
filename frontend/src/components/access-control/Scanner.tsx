import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router";
import QrScanner from "qr-scanner";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { toast } from "sonner";
import { AccessDirection, ScanResult } from "@/common/types";
import {
  useGetAccessEvent,
  useManualAccessEntry,
  useScanAccessPass,
  useSearchAccessMembers,
} from "@/lib/api";
import { useLanguageSync } from "@/i18n/useLanguageSync";
import { cn, describeError, useDebounce } from "@/lib/utils";
import { LanguageSwitcher } from "../language-switcher/LanguageSwitcher";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { localDate, localTime, statusKey } from "./labels";

/** How long a result stays on screen before the next scan. */
const RESULT_MS = 2500;

type Tone = "ok" | "warn" | "error";

function describe(
  result: ScanResult,
  direction: AccessDirection,
  t: (key: string, opts?: Record<string, string>) => string,
): { tone: Tone; text: string } {
  const o = result.outcome;
  switch (o.kind) {
    case "recorded":
      return {
        tone: "ok",
        text: t(
          direction === "in"
            ? "access.result_recorded_in"
            : "access.result_recorded_out",
        ),
      };
    case "already_inside":
      return {
        tone: "warn",
        text: t("access.result_already_inside", { time: localTime(o.since) }),
      };
    case "not_inside":
      return { tone: "warn", text: t("access.result_not_inside") };
    case "not_eligible":
      return {
        tone: "error",
        text: o.ended_on
          ? t("access.result_not_eligible_ended", {
              date: localDate(o.ended_on),
            })
          : t("access.result_not_eligible"),
      };
    case "expired_pass":
      return { tone: "error", text: t("access.result_expired_pass") };
    case "invalid_pass":
      return { tone: "error", text: t("access.result_invalid_pass") };
    case "event_not_active":
      return { tone: "error", text: t("access.result_event_not_active") };
  }
}

const toneClass: Record<Tone, string> = {
  ok: "bg-green-600 text-white",
  warn: "bg-yellow-400 text-black",
  error: "bg-red-600 text-white",
};

/** Each mode has its own colour so staff see at a glance which one is on. */
const modeClass: Record<AccessDirection, string> = {
  in: "bg-green-600 text-white hover:bg-green-600/90",
  out: "bg-orange-600 text-white hover:bg-orange-600/90",
};

/// The door: pick in or out, scan member passes with the camera, or find a
/// member by name. Each result shows big for a moment.
export default function Scanner() {
  const { t } = useTranslation();
  useLanguageSync();
  const { id = "" } = useParams<{ id: string }>();
  const { data } = useGetAccessEvent(id, 10_000);
  const scan = useScanAccessPass(id);
  const manual = useManualAccessEntry(id);

  const [direction, setDirection] = useState<AccessDirection>("in");
  const [shown, setShown] = useState<{
    result: ScanResult;
    direction: AccessDirection;
  } | null>(null);
  const [cameraError, setCameraError] = useState(false);
  const [cameraReady, setCameraReady] = useState(false);
  const [query, setQuery] = useState("");
  const debounced = useDebounce(query, 300);
  const { data: matches } = useSearchAccessMembers(id, debounced);

  const videoRef = useRef<HTMLVideoElement>(null);
  // Read by the camera callback, which is set up once.
  const directionRef = useRef(direction);
  directionRef.current = direction;
  const busyRef = useRef(false);
  const lastTokenRef = useRef<string | null>(null);
  const hideTimerRef = useRef<number | undefined>(undefined);

  const hide = () => {
    window.clearTimeout(hideTimerRef.current);
    setShown(null);
    busyRef.current = false;
  };

  const show = (result: ScanResult, dir: AccessDirection) => {
    setShown({ result, direction: dir });
    if ("vibrate" in navigator) navigator.vibrate(80);
    window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(hide, RESULT_MS);
  };

  useEffect(() => () => window.clearTimeout(hideTimerRef.current), []);

  const active =
    data?.event.status === "running" || data?.event.status === "paused";

  useEffect(() => {
    if (!active || !videoRef.current) return;
    const scanner = new QrScanner(
      videoRef.current,
      (decoded) => {
        const token = decoded.data;
        // The same pass stays in view after a scan; don't record it twice.
        if (busyRef.current || token === lastTokenRef.current) return;
        busyRef.current = true;
        lastTokenRef.current = token;
        const dir = directionRef.current;
        scan.mutate(
          { token, direction: dir },
          {
            onSuccess: (result) => show(result, dir),
            onError: (e) => {
              toast.error(describeError(e));
              busyRef.current = false;
            },
          },
        );
      },
      {
        preferredCamera: "environment",
        highlightScanRegion: true,
        maxScansPerSecond: 5,
        returnDetailedScanResult: true,
      },
    );
    scanner
      .start()
      .then(() => setCameraReady(true))
      .catch(() => setCameraError(true));
    return () => {
      scanner.destroy();
    };
    // scan.mutate is stable enough; restarting the camera on every render
    // would make it flicker.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  // A new mode means the pass in view may be scanned again.
  useEffect(() => {
    lastTokenRef.current = null;
  }, [direction]);

  const shownText = shown && describe(shown.result, shown.direction, t);

  return (
    <div className="max-w-lg mx-auto px-4 pb-[max(1rem,env(safe-area-inset-bottom))] space-y-4">
      {/* Stays on top while scrolling to the manual search, so the mode is
          always visible and one tap away. */}
      <div className="sticky top-0 z-10 -mx-4 px-4 pt-[max(0.75rem,env(safe-area-inset-top))] pb-3 space-y-3 bg-background border-b">
        <div className="flex items-center justify-between gap-2">
          <Link
            to={`/access-control/${id}`}
            className="inline-flex items-center min-h-10 text-sm text-muted-foreground"
          >
            <ArrowLeft className="h-4 w-4 mr-1" /> {t("access.back_to_event")}
          </Link>
          <LanguageSwitcher />
        </div>

        <div className="flex items-baseline justify-between gap-3">
          <h1 className="text-xl font-bold truncate">
            {data?.event.name ?? "…"}
          </h1>
          {data && (
            <p className="shrink-0 text-sm text-muted-foreground">
              {t(statusKey[data.event.status])}
            </p>
          )}
        </div>
        {data && (
          <p className="text-lg font-semibold -mt-2">
            {t("access.present_count", {
              count: String(data.present.length),
            })}
          </p>
        )}

        <div className="grid grid-cols-2 gap-2">
          {(["in", "out"] as AccessDirection[]).map((d) => (
            <Button
              key={d}
              size="lg"
              aria-pressed={direction === d}
              className={cn(
                "h-14 text-xl font-bold",
                direction === d && modeClass[d],
              )}
              variant={direction === d ? "default" : "outline"}
              onClick={() => setDirection(d)}
            >
              {t(d === "in" ? "access.mode_in" : "access.mode_out")}
            </Button>
          ))}
        </div>
      </div>

      {!active && data && (
        <p className="text-muted-foreground">
          {t("access.event_not_active_note")}
        </p>
      )}

      {active && (
        // Square, but never taller than about half the screen, so the
        // manual search stays reachable on short phones and in landscape.
        <div className="relative mx-auto w-full max-w-[55dvh] aspect-square rounded-lg overflow-hidden bg-black">
          <video ref={videoRef} className="w-full h-full object-cover" muted />
          <span
            className={cn(
              "absolute top-2 left-2 rounded-md px-3 py-1 text-base font-bold",
              modeClass[direction],
            )}
          >
            {t(direction === "in" ? "access.mode_in" : "access.mode_out")}
          </span>
          {!cameraReady && !cameraError && (
            <p className="absolute inset-0 flex items-center justify-center text-white">
              {t("access.camera_starting")}
            </p>
          )}
          {cameraError && (
            <p className="absolute inset-0 flex items-center justify-center p-6 text-center text-white">
              {t("access.camera_error")}
            </p>
          )}
        </div>
      )}

      {active && (
        <div className="space-y-2">
          <h2 className="font-semibold">{t("access.manual_title")}</h2>
          <Input
            type="search"
            inputMode="search"
            enterKeyHint="search"
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            // 16px text keeps iOS from zooming in on focus.
            className="h-12 text-base"
            value={query}
            placeholder={t("access.manual_search")}
            onChange={(e) => setQuery(e.target.value)}
          />
          {debounced.trim().length >= 2 && matches?.length === 0 && (
            <p className="text-sm text-muted-foreground">
              {t("access.manual_no_results")}
            </p>
          )}
          {matches?.map((m) => (
            <div
              key={m.member.user_id}
              className="flex items-center justify-between gap-3 border rounded-md p-3"
            >
              <div className="min-w-0">
                <p className="font-medium truncate">{m.member.name}</p>
                <p className="text-sm text-muted-foreground truncate">
                  {m.member.email}
                </p>
                {(m.inside || !m.eligible) && (
                  <p className="text-sm font-medium">
                    {m.inside && t("access.manual_inside")}
                    {m.inside && !m.eligible && " · "}
                    {!m.eligible && (
                      <span className="text-red-600">
                        {t("access.manual_no_role")}
                      </span>
                    )}
                  </p>
                )}
              </div>
              <Button
                size="lg"
                className={cn(
                  "h-12 min-w-20 px-4 text-base font-bold",
                  modeClass[direction],
                )}
                disabled={manual.isPending}
                onClick={() => {
                  // Drop the keyboard so the result is seen in full.
                  (document.activeElement as HTMLElement | null)?.blur();
                  manual.mutate(
                    { user_id: m.member.user_id, direction },
                    {
                      onSuccess: (result) => {
                        busyRef.current = true;
                        show(result, direction);
                        setQuery("");
                      },
                      onError: (e) => toast.error(describeError(e)),
                    },
                  );
                }}
              >
                {t(direction === "in" ? "access.mode_in" : "access.mode_out")}
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* Covers the whole screen, not just the camera, so a result from the
          manual search is seen even when the camera is scrolled away. Tap to
          go on to the next member straight away. */}
      {shown && shownText && (
        <button
          type="button"
          onClick={hide}
          className={cn(
            "fixed inset-0 z-50 flex flex-col items-center justify-center p-6 text-center",
            toneClass[shownText.tone],
          )}
        >
          <p className="text-5xl font-bold">{shownText.text}</p>
          {shown.result.member && (
            <p className="text-3xl mt-4">{shown.result.member.name}</p>
          )}
        </button>
      )}
    </div>
  );
}
