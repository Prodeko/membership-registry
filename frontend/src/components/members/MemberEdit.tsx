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
import { useGetMember, useUpdateMember, QueryKey } from "@/lib/api";
import { COUNTRIES, FINNISH_MUNICIPALITIES } from "@/lib/constants";
import { zodResolver } from "@hookform/resolvers/zod";
import { useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { z } from "zod";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import MunicipalitySelect from "../ui/MunicipalitySelect";

const MemberEdit = () => {
  const { id: userId } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const formSchema = z.object({
    first_name: z.string().min(1, t("validation.first_name_required")),
    last_name: z.string().min(1, t("validation.last_name_required")),
    email: z.string().email(),
    home_municipality: z
      .enum([...FINNISH_MUNICIPALITIES, ...COUNTRIES])
      .optional(),
  });

  type MemberEditFormValues = z.infer<typeof formSchema>;

  const { data: member, isLoading } = useGetMember(userId!);
  const { mutate: updateMember, isPending } = useUpdateMember();

  const form = useForm<MemberEditFormValues>({
    resolver: zodResolver(formSchema),
    values: member
      ? {
          first_name: member.first_name,
          last_name: member.last_name,
          email: member.email,
          home_municipality:
            (member.home_municipality as MemberEditFormValues["home_municipality"]) ??
            undefined,
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

  const onSubmit = (values: MemberEditFormValues) => {
    updateMember(
      {
        userId: member.user_id,
        data: {
          first_name: values.first_name,
          last_name: values.last_name,
          email: values.email,
          home_municipality: values.home_municipality ?? null,
          email_notifications: member.email_notifications,
          language: member.language,
        },
      },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({
            queryKey: [QueryKey.MEMBER, { id: userId }],
          });
          navigate(`/members/${userId}`);
        },
      },
    );
  };

  return (
    <main className="flex flex-col items-center min-h-screen w-screen px-4 py-20 gap-4">
      <Card className="p-10 space-y-4 h-fit max-w-lg w-full">
        <h1 className="text-2xl font-bold">{t("members.edit.title")}</h1>
        <div className="text-sm text-muted-foreground">
          <span className="font-medium">{t("members.edit.id_label")}: </span>
          {member.user_id}
        </div>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="first_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t("profile.fields.first_name")}</FormLabel>
                  <FormControl>
                    <Input {...field} />
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
                    <Input {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="email"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t("profile.fields.email")}</FormLabel>
                  <FormControl>
                    <Input type="email" {...field} />
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
            <div className="flex gap-2">
              <Button type="submit" disabled={isPending}>
                {isPending
                  ? t("members.edit.saving")
                  : t("members.edit.save_button")}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => navigate(`/members/${userId}`)}
              >
                {t("common.cancel")}
              </Button>
            </div>
          </form>
        </Form>
      </Card>
    </main>
  );
};

export default MemberEdit;
