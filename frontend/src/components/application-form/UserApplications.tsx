import { useGetTargetableRoles, useGetUserApplications } from "@/lib/api";
import { kebabCaseToTitleCase } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "../ui/accordion";
import { Badge } from "../ui/badge";
import { Separator } from "../ui/separator";
import InfoTooltip from "../ui/info-tooltip";

const UserApplications = () => {
  const { t } = useTranslation();
  const { data: userApplications } = useGetUserApplications();
  const { data: targetableRoles } = useGetTargetableRoles();

  if (!userApplications || !targetableRoles) {
    return null;
  }

  return (
    <Accordion type="single" collapsible>
      <AccordionItem value="item-1">
        <AccordionTrigger className="font-bold">
          <span>
            {t("applications.existing.title")}
            <InfoTooltip>
              <p>{t("applications.existing.tooltip")}</p>
            </InfoTooltip>
          </span>
        </AccordionTrigger>
        <AccordionContent className="space-y-4">
          {userApplications.map((app) => {
            const role = targetableRoles.find(
              (role) =>
                role.role_name === app.role_name &&
                role.valid_until === app.valid_until,
            );
            return (
              <>
                <Separator />
                <div
                  key={app.application_id}
                  className="flex justify-between w-full"
                >
                  <div>
                    <h4 className="text-md font-semibold">
                      {t("applications.existing.role_validity", {
                        role_name: kebabCaseToTitleCase(app.role_name),
                        date: app.valid_until,
                      })}
                    </h4>
                    <div className="text-sm">
                      {t("applications.existing.status_label")}
                      <Badge
                        variant={
                          app.status === "approved"
                            ? "default"
                            : app.status === "rejected"
                              ? "destructive"
                              : "secondary"
                        }
                        className="ml-2"
                      >
                        {t(`status.${app.status}`)}
                      </Badge>
                    </div>
                  </div>
                  {role?.payment_link && app.status === "unpaid" && (
                    <Badge
                      variant={"secondary"}
                      className="text-sm align-center"
                    >
                      <a
                        href={role.payment_link}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-500 hover:underline"
                      >
                        {t("applications.existing.pay_fee")}
                      </a>
                    </Badge>
                  )}
                </div>
              </>
            );
          })}
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
};

export default UserApplications;
