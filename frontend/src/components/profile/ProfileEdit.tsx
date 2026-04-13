import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { useGetMeMember, useUpdateMember } from "@/lib/api";
import { COUNTRIES, FINNISH_MUNICIPALITIES } from "@/lib/constants";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { z } from "zod";
import MarketingPreferences from "./MarketingPreferences";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Switch } from "../ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import MunicipalitySelect from "../ui/MunicipalitySelect";

const ProfileEdit = () => {
  const { t } = useTranslation();

  const formSchema = z.object({
    first_name: z.string().min(1, t("validation.first_name_required")),
    last_name: z.string().min(1, t("validation.last_name_required")),
    home_municipality: z
      .enum([...FINNISH_MUNICIPALITIES, ...COUNTRIES])
      .optional(),
    email_notifications: z.boolean(),
    language: z.enum(["fi", "en"]),
  });

  type ProfileFormValues = z.infer<typeof formSchema>;

  const { data: member, isLoading } = useGetMeMember();
  const { mutate: updateMember, isPending } = useUpdateMember();
  const navigate = useNavigate();

  const form = useForm<ProfileFormValues>({
    resolver: zodResolver(formSchema),
    values: member
      ? {
          first_name: member.first_name,
          last_name: member.last_name,
          home_municipality:
            (member.home_municipality as ProfileFormValues["home_municipality"]) ??
            undefined,
          email_notifications: member.email_notifications,
          language: (member.language as "fi" | "en") || "fi",
        }
      : undefined,
  });

  if (isLoading || !member) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">{t("common.loading")}</p>
      </main>
    );
  }

  const onSubmit = (values: ProfileFormValues) => {
    updateMember(
      {
        userId: member.user_id,
        data: {
          ...values,
          home_municipality: values.home_municipality ?? null,
        },
      },
      {
        onSuccess: () => {
          navigate("/home");
        },
      },
    );
  };

  return (
    <main className="flex flex-col items-center min-h-screen w-screen px-4 py-20 gap-4">
      <Card className="p-10 space-y-4 h-fit max-w-lg w-full">
        <h1 className="text-2xl font-bold">{t("profile.edit.title")}</h1>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="first_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t("profile.fields.first_name")}</FormLabel>
                  <FormControl>
                    <Input data-testid="profile-first-name" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="last_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t("profile.fields.last_name")}</FormLabel>
                  <FormControl>
                    <Input data-testid="profile-last-name" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="home_municipality"
              render={({ field }) => (
                <FormItem className="flex flex-col">
                  <FormLabel>{t("profile.fields.home_municipality")}</FormLabel>
                  <MunicipalitySelect field={field} form={form} />
                  <FormDescription>
                    {t("profile.municipality_description")}
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="language"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t("profile.fields.language")}</FormLabel>
                  <Select
                    onValueChange={field.onChange}
                    defaultValue={field.value}
                  >
                    <FormControl>
                      <SelectTrigger data-testid="profile-language-select">
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="fi">{t("language.fi")}</SelectItem>
                      <SelectItem value="en">{t("language.en")}</SelectItem>
                    </SelectContent>
                  </Select>
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="email_notifications"
              render={({ field }) => (
                <FormItem className="flex flex-row items-center justify-between rounded-lg border p-4">
                  <div className="space-y-0.5">
                    <FormLabel className="text-base">
                      {t("profile.fields.email_notifications")}
                    </FormLabel>
                    <FormDescription>
                      {t("profile.notifications_description")}
                    </FormDescription>
                  </div>
                  <FormControl>
                    <Switch
                      checked={field.value}
                      onCheckedChange={field.onChange}
                      data-testid="profile-notifications-switch"
                    />
                  </FormControl>
                </FormItem>
              )}
            />
            <div className="flex gap-2">
              <Button
                type="submit"
                disabled={isPending}
                data-testid="profile-save-button"
              >
                {isPending
                  ? t("profile.edit.saving")
                  : t("profile.edit.save_button")}
              </Button>
              <Button
                type="button"
                variant="outline"
                data-testid="profile-cancel-button"
                onClick={() => navigate("/home")}
              >
                {t("common.cancel")}
              </Button>
            </div>
          </form>
        </Form>
      </Card>
      <div className="max-w-lg w-full">
        <MarketingPreferences />
      </div>
    </main>
  );
};

export default ProfileEdit;
