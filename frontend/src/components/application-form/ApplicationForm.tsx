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
  useGetMyAttributes,
  useGetTargetableRoles,
  useGetUserApplications,
} from "@/lib/api";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { z } from "zod";
import MultiValueSelect from "../attributes/MultiValueSelect";
import RenderMemberData from "../members/RenderMemberData";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
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

type FormValues = z.infer<typeof formSchema>;

const ApplicationForm = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: currentMember, isLoading: isMeLoading } = useGetMeMember();

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      application_text: "",
    },
  });

  const { data: targetableRoles, isLoading: isRolesLoading } =
    useGetTargetableRoles();
  const { data: applications } = useGetUserApplications();
  const { data: memberAttributes } = useGetMyAttributes();
  const { mutateAsync: createApplication } = useCreateApplication();
  const [paymentLink, setPaymentLink] = useState<string | null>(null);
  const [attributeValues, setAttributeValues] = useState<
    Record<string, string[]>
  >({});

  const selectedRole = form.watch("role_name");
  const selectedTargetable = useMemo(
    () => targetableRoles?.find((r) => r.role_name === selectedRole),
    [targetableRoles, selectedRole],
  );

  // Look each form attribute up against the user's attribute catalog so we
  // can render the right input shape (free-text vs allowed_values dropdown)
  // and pull description copy. Falls back to a plain text input if the
  // catalog hasn't loaded yet.
  const formAttributeDefs = useMemo(() => {
    const names = selectedTargetable?.form_attributes ?? [];
    return names.map((name) => {
      const def = memberAttributes?.find((a) => a.name === name);
      return {
        name,
        description: def?.description ?? null,
        allowed_values: def?.allowed_values ?? null,
        required: def?.required ?? false,
        multiple: def?.multiple ?? false,
        currentValues: def?.values ?? [],
      };
    });
  }, [selectedTargetable, memberAttributes]);

  const missingRequired = formAttributeDefs.filter(
    ({ name, required, currentValues }) =>
      required && (attributeValues[name] ?? currentValues).length === 0,
  );

  const onSubmit = async (values: FormValues) => {
    if (!currentMember) {
      throw new Error("Current member not found");
    }
    if (missingRequired.length > 0) return;

    const submittedAttributes = formAttributeDefs
      .map(({ name, currentValues }) => ({
        name,
        values: attributeValues[name] ?? currentValues,
      }))
      .filter((kv) => kv.values.length > 0);

    try {
      const data = await createApplication({
        role_name: values.role_name,
        valid_until: values.valid_until.toISOString().split("T")[0],
        application_text: values.application_text,
        stripe_payment_id: null,
        optional_roles: null,
        attributes:
          submittedAttributes.length > 0 ? submittedAttributes : undefined,
      });

      const redirectUrl = new URL(data.redirect_to);
      if (redirectUrl.origin === window.location.origin) {
        navigate(redirectUrl.pathname + redirectUrl.search);
      } else {
        window.location.href = data.redirect_to;
      }
    } catch {
      // Errors are shown via the global MutationCache onError toast
    }
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
                      <SelectTrigger data-testid="role-select">
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
            {formAttributeDefs.length > 0 && (
              <div className="space-y-3">
                {formAttributeDefs.map((attr) => {
                  const values =
                    attributeValues[attr.name] ?? attr.currentValues;
                  const setValues = (vs: string[]) =>
                    setAttributeValues((prev) => ({
                      ...prev,
                      [attr.name]: vs,
                    }));
                  const value = values[0] ?? "";
                  const set = (v: string) => setValues(v ? [v] : []);
                  const id = `application-attr-${attr.name}`;
                  return (
                    <div key={attr.name} className="space-y-1">
                      <Label htmlFor={id} className="font-mono text-sm">
                        {attr.name}
                        {attr.required && (
                          <span
                            className="ml-0.5 text-destructive"
                            title={t("attributes.required")}
                          >
                            *
                          </span>
                        )}
                      </Label>
                      {attr.description && (
                        <p className="text-xs text-muted-foreground">
                          {attr.description}
                        </p>
                      )}
                      {attr.multiple ? (
                        <MultiValueSelect
                          id={id}
                          values={values}
                          allowedValues={attr.allowed_values}
                          onChange={setValues}
                          data-testid={`application-attr-${attr.name}`}
                        />
                      ) : attr.allowed_values &&
                        attr.allowed_values.length > 0 ? (
                        <Select value={value} onValueChange={set}>
                          <SelectTrigger
                            id={id}
                            className="w-64"
                            data-testid={`application-attr-${attr.name}`}
                          >
                            <SelectValue placeholder="(select)" />
                          </SelectTrigger>
                          <SelectContent>
                            {attr.allowed_values.map((v) => (
                              <SelectItem key={v} value={v}>
                                {v}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      ) : (
                        <Input
                          id={id}
                          value={value}
                          onChange={(e) => set(e.target.value)}
                          data-testid={`application-attr-${attr.name}`}
                        />
                      )}
                    </div>
                  );
                })}
              </div>
            )}
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
                      data-testid="application-text"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            {missingRequired.length > 0 && (
              <p
                className="text-sm text-destructive"
                data-testid="application-required-missing"
              >
                {t("application.form.required_missing", {
                  names: missingRequired.map((a) => a.name).join(", "),
                })}
              </p>
            )}
            <Button
              type="submit"
              disabled={missingRequired.length > 0}
              data-testid="submit-application-button"
            >
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
