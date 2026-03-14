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
import { useNavigate } from "react-router-dom";
import { z } from "zod";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Switch } from "../ui/switch";
import MunicipalitySelect from "../signup-form/MunicipalitySelect";

const formSchema = z.object({
  first_name: z.string().min(1, "First name is required"),
  last_name: z.string().min(1, "Last name is required"),
  home_municipality: z.enum([...FINNISH_MUNICIPALITIES, ...COUNTRIES]),
  email_notifications: z.boolean(),
});

type ProfileFormValues = z.infer<typeof formSchema>;

const ProfileEdit = () => {
  const { data: member, isLoading } = useGetMeMember();
  const { mutate: updateMember, isPending } = useUpdateMember();
  const navigate = useNavigate();

  const form = useForm<ProfileFormValues>({
    resolver: zodResolver(formSchema),
    values: member
      ? {
          first_name: member.first_name,
          last_name: member.last_name,
          home_municipality: member.home_municipality as ProfileFormValues["home_municipality"],
          email_notifications: member.email_notifications,
        }
      : undefined,
  });

  if (isLoading || !member) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">Loading...</p>
      </main>
    );
  }

  const onSubmit = (values: ProfileFormValues) => {
    updateMember(
      {
        userId: member.user_id,
        data: {
          ...values,
          has_accepted_policies: member.has_accepted_policies,
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
    <main className="flex justify-center min-h-screen w-screen px-4 py-20">
      <Card className="p-10 space-y-4 h-fit max-w-lg w-full">
        <h1 className="text-2xl font-bold">Edit profile</h1>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="first_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>First name</FormLabel>
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
                  <FormLabel>Last name</FormLabel>
                  <FormControl>
                    <Input {...field} />
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
                  <FormLabel>Home municipality</FormLabel>
                  <MunicipalitySelect field={field} form={form} />
                  <FormDescription>
                    Select the municipality where you mainly{" "}
                    <strong>live</strong>
                    <br />
                    If you mainly live outside of Finland, select your country.
                  </FormDescription>
                  <FormMessage />
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
                      Email notifications
                    </FormLabel>
                    <FormDescription>
                      Receive email notifications about application status
                      changes.
                    </FormDescription>
                  </div>
                  <FormControl>
                    <Switch
                      checked={field.value}
                      onCheckedChange={field.onChange}
                    />
                  </FormControl>
                </FormItem>
              )}
            />
            <div className="flex gap-2">
              <Button type="submit" disabled={isPending}>
                {isPending ? "Saving..." : "Save changes"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => navigate("/home")}
              >
                Cancel
              </Button>
            </div>
          </form>
        </Form>
      </Card>
    </main>
  );
};

export default ProfileEdit;
