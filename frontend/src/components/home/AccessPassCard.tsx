import { QRCodeSVG } from "qrcode.react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { DoorOpen, QrCode } from "lucide-react";
import { useGetAccessPass } from "@/lib/api";
import { localDate, statusKey } from "../access-control/labels";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card, CardContent } from "../ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";

/// The entry pass for meetings, shown while an event is running or on a
/// break, plus a link to the door pages for door staff.
export default function AccessPassCard({ name }: { name: string }) {
  const { t } = useTranslation();
  const { data } = useGetAccessPass();
  const [open, setOpen] = useState(false);

  if (!data || (data.events.length === 0 && !data.can_control)) {
    return null;
  }

  return (
    <Card>
      <CardContent className="pt-6 space-y-3">
        {data.token && (
          <Button
            size="lg"
            className="w-full h-14 text-lg"
            onClick={() => setOpen(true)}
          >
            <QrCode className="h-6 w-6 mr-2" />
            {t("access.pass_button")}
          </Button>
        )}
        {data.events.map((event) => (
          <div
            key={event.id}
            className="flex items-center justify-between gap-2 text-sm"
          >
            <span>
              {event.eligible
                ? event.name
                : event.ended_on
                  ? t("access.pass_not_eligible_ended", {
                      name: event.name,
                      date: localDate(event.ended_on),
                    })
                  : t("access.pass_not_eligible", { name: event.name })}
            </span>
            <Badge variant={event.status === "paused" ? "outline" : "default"}>
              {t(statusKey[event.status])}
            </Badge>
          </div>
        ))}
        {data.can_control && (
          <Button asChild variant="outline" className="w-full">
            <Link to="/access-control">
              <DoorOpen className="h-4 w-4 mr-2" />
              {t("access.control_link")}
            </Link>
          </Button>
        )}
      </CardContent>

      <Dialog open={open && !!data.token} onOpenChange={setOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{t("access.pass_title")}</DialogTitle>
            <DialogDescription>
              {t("access.pass_description")}
            </DialogDescription>
          </DialogHeader>
          {data.token && (
            <div className="flex flex-col items-center gap-3">
              {/* White quiet zone so scanners read it in dark mode too. */}
              <div className="bg-white p-4 rounded-lg">
                <QRCodeSVG value={data.token} size={260} level="M" />
              </div>
              <p className="text-xl font-semibold">{name}</p>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </Card>
  );
}
