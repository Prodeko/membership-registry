import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import {
  useCreateApplication,
  useGetMeMember,
  useGetTargetableRoles,
  useGetUserApplications,
} from "@/lib/api";
import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { z } from "zod";
import RenderMemberData from "../members/RenderMemberData";
import { Card } from "../ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { Separator } from "../ui/separator";
import { Textarea } from "../ui/textarea";
import UserApplications from "./UserApplications";
import InfoTooltip from "../ui/info-tooltip";
import { kebabCaseToTitleCase } from "@/lib/utils";

const formSchema = z.object({
  application_text: z.string(),
  role_name: z.string(),
  valid_until: z.date(),
});

export type ApplicationFormValues = z.infer<typeof formSchema>;

const ApplicationForm = () => {
  const { t } = useTranslation();
  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
  });

  const { data: currentMember, isLoading: isMeLoading } = useGetMeMember();
  const { data: targetableRoles, isLoading: isRolesLoading } =
    useGetTargetableRoles();
  const { data: applications } = useGetUserApplications();
  const { mutate: createApplication } = useCreateApplication();
  const [paymentLink, setPaymentLink] = useState<string | null>(null);

  const onSubmit = (values: z.infer<typeof formSchema>) => {
    if (!currentMember) {
      throw new Error("Current member not found");
    }

    createApplication(
      {
        role_name: values.role_name,
        valid_until: values.valid_until.toISOString().split("T")[0],
        application_text: values.application_text,
        stripe_payment_id: null,
        optional_roles: null,
      },
      {
        onSuccess: (data) => {
          window.location.href = data.redirect_to;
        },
      },
    );
  };

  if (isRolesLoading || isMeLoading) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">{t("common.loading")}</p>
      </main>
    );
  }

  if (!targetableRoles) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">{t("errors.no_roles")}</p>
      </main>
    );
  }

  if (!currentMember) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">
          {t("errors.profile_load_failed")}
        </p>
      </main>
    );
  }

  return (
    <main className="flex justify-center align-middle h-screen w-screen py-20 px-4">
      <Card className="p-10 space-y-4 h-fit">
        <h1 className="text-4xl">{t("application.form.title")}</h1>
        <Separator />
        <RenderMemberData member={currentMember} variant="enduser" />
        <div>{t("application.form.instructions")}</div>
        <UserApplications />
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="role_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>
                    {t("application.form.membership_type_label")}
                    <InfoTooltip>
                      <p>{t("application.form.membership_tooltip")}</p>
                    </InfoTooltip>
                  </FormLabel>
                  <Select
                    onValueChange={(value) => {
                      const targetableRole = targetableRoles?.find(
                        (role) => role.role_name === value,
                      );
                      const validUntil = targetableRole?.valid_until;
                      const paymentLink = targetableRole?.payment_link;

                      if (!validUntil) {
                        throw new Error(
                          `Role ${value} not found in targetable roles`,
                        );
                      }

                      setPaymentLink(paymentLink ?? null);
                      form.setValue("valid_until", new Date(validUntil));
                      return field.onChange(value);
                    }}
                  >
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue
                          placeholder={t("application.form.role_placeholder")}
                        />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      {targetableRoles?.map((role) => {
                        const existing = applications?.find(
                          (app) =>
                            app.status !== "rejected" &&
                            app.role_name === role.role_name &&
                            app.valid_until === role.valid_until,
                        );
                        return (
                          <SelectItem
                            value={role.role_name}
                            key={role.role_name + role.valid_until}
                            disabled={!!existing}
                          >
                            {kebabCaseToTitleCase(role.role_name)}{" "}
                            <span>
                              {t("application.form.valid_until_text", {
                                date: new Date(
                                  role.valid_until,
                                ).toLocaleDateString(),
                              })}
                            </span>
                          </SelectItem>
                        );
                      })}
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="application_text"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>
                    {t("application.form.application_text_label")}
                  </FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder={t(
                        "application.form.application_text_placeholder",
                      )}
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <Button type="submit">
              {paymentLink
                ? t("application.form.proceed_payment")
                : t("application.form.submit")}
            </Button>
          </form>
        </Form>
      </Card>
    </main>
  );
};

export default ApplicationForm;
