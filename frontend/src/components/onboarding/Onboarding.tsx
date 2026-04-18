import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
  Form,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import MunicipalitySelect from "@/components/ui/MunicipalitySelect";
import { useGetMeMember, useUpdateMember } from "@/lib/api";
import { COUNTRIES, FINNISH_MUNICIPALITIES } from "@/lib/constants";
import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { z } from "zod";

const Onboarding = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: member, isLoading } = useGetMeMember();
  const { mutate: updateMember, isPending } = useUpdateMember();

  const formSchema = z.object({
    home_municipality: z.enum([...FINNISH_MUNICIPALITIES, ...COUNTRIES], {
      message: t("validation.home_municipality_required"),
    }),
  });
  type FormValues = z.infer<typeof formSchema>;

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
  });

  useEffect(() => {
    if (member?.home_municipality) {
      navigate("/home", { replace: true });
    }
  }, [member, navigate]);

  if (isLoading || !member) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">{t("common.loading")}</p>
      </main>
    );
  }

  const onSubmit = (values: FormValues) => {
    updateMember(
      {
        userId: member.user_id,
        data: {
          first_name: member.first_name,
          last_name: member.last_name,
          home_municipality: values.home_municipality,
          email_notifications: member.email_notifications,
          language: member.language,
          email: null,
        },
      },
      {
        onSuccess: () => {
          navigate("/home", { replace: true });
        },
      },
    );
  };

  return (
    <main className="flex justify-center min-h-screen w-screen px-4 py-20">
      <Card className="p-10 space-y-4 h-fit max-w-lg w-full">
        <div className="space-y-2">
          <h1 className="text-2xl font-bold">{t("onboarding.title")}</h1>
          <p className="text-muted-foreground">{t("onboarding.intro")}</p>
        </div>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="home_municipality"
              render={({ field }) => (
                <FormItem className="flex flex-col">
                  <FormLabel>{t("profile.fields.home_municipality")}</FormLabel>
                  <MunicipalitySelect field={field} form={form} />
                  <FormDescription>
                    {t("onboarding.municipality_description")}
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />
            <Button
              type="submit"
              disabled={isPending}
              data-testid="onboarding-submit-button"
            >
              {isPending ? t("common.loading") : t("onboarding.submit")}
            </Button>
          </form>
        </Form>
      </Card>
    </main>
  );
};

export default Onboarding;
