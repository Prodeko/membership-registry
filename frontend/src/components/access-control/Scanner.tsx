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

  const show = (result: ScanResult, dir: AccessDirection) => {
    setShown({ result, direction: dir });
    if ("vibrate" in navigator) navigator.vibrate(80);
    window.setTimeout(() => {
      setShown(null);
      busyRef.current = false;
    }, RESULT_MS);
  };

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
    <div className="max-w-lg mx-auto p-4 space-y-4">
      <div className="flex items-center justify-between">
        <Link
          to={`/access-control/${id}`}
          className="inline-flex items-center text-sm text-muted-foreground"
        >
          <ArrowLeft className="h-4 w-4 mr-1" /> {t("access.back_to_event")}
        </Link>
        <LanguageSwitcher />
      </div>

      <div>
        <h1 className="text-2xl font-bold">{data?.event.name ?? "…"}</h1>
        {data && (
          <p className="text-muted-foreground">
            {t(statusKey[data.event.status])} ·{" "}
            {t("access.present_count", {
              count: String(data.present.length),
            })}
          </p>
        )}
      </div>

      <div className="grid grid-cols-2 gap-2">
        {(["in", "out"] as AccessDirection[]).map((d) => (
          <Button
            key={d}
            size="lg"
            className="h-14 text-lg"
            variant={direction === d ? "default" : "outline"}
            onClick={() => setDirection(d)}
          >
            {t(d === "in" ? "access.mode_in" : "access.mode_out")}
          </Button>
        ))}
      </div>

      {!active && data && (
        <p className="text-muted-foreground">
          {t("access.event_not_active_note")}
        </p>
      )}

      {active && (
        <div className="relative rounded-lg overflow-hidden bg-black aspect-square">
          <video ref={videoRef} className="w-full h-full object-cover" muted />
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
          {shown && shownText && (
            <div
              className={cn(
                "absolute inset-0 flex flex-col items-center justify-center p-6 text-center",
                toneClass[shownText.tone],
              )}
            >
              <p className="text-4xl font-bold">{shownText.text}</p>
              {shown.result.member && (
                <p className="text-2xl mt-3">{shown.result.member.name}</p>
              )}
            </div>
          )}
        </div>
      )}

      {active && (
        <div className="space-y-2">
          <h2 className="font-semibold">{t("access.manual_title")}</h2>
          <Input
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
              className="flex items-center justify-between gap-2 border rounded-md p-2"
            >
              <div className="min-w-0">
                <p className="font-medium truncate">{m.member.name}</p>
                <p className="text-xs text-muted-foreground truncate">
                  {m.member.email}
                  {m.inside && ` · ${t("access.manual_inside")}`}
                  {!m.eligible && ` · ${t("access.manual_no_role")}`}
                </p>
              </div>
              <Button
                disabled={manual.isPending}
                onClick={() =>
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
                  )
                }
              >
                {t(direction === "in" ? "access.mode_in" : "access.mode_out")}
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
