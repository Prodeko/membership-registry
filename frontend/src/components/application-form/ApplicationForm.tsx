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
  useGetTargetableRoles
} from "@/lib/api";
import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import RenderMemberData from "../members/RenderUserData";
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

const formSchema = z.object({
  application_text: z.string({
    required_error: "Application text is required",
  }),
  role_name: z.string({ required_error: "Role is required" }),
  valid_until: z.date({ required_error: "Valid until is required" }),
});

export type ApplicationFormValues = z.infer<typeof formSchema>;

const ApplicationForm = () => {
  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
  });

  const { data: currentMember, isLoading: isMeLoading } = useGetMeMember();
  const { data: targetableRoles, isLoading: isRolesLoading } = useGetTargetableRoles();
  const { mutate: createApplication } = useCreateApplication();
  const [paymentLink, setPaymentLink] = useState<string | null>(null);

  const onSubmit = (values: z.infer<typeof formSchema>) => {
    if (!currentMember) {
      throw new Error("Current member not found");
    }

    createApplication({
      ...values,
      user_id: currentMember?.user_id,
    }, {
      onSuccess: (data: {redirect_to: string}) => {
        window.location.href = data.redirect_to // TODO: Is this ok?
      }
    });
  };

  if (isRolesLoading || isMeLoading) {
    return <div>Loading...</div>;
  }

  if (!targetableRoles) {
    return <div>No roles to apply to!</div>;
  }

  if (!currentMember) {
    return <div>No member found!</div>;
  }

  return (
    <main className="flex justify-center align-middle h-screen w-screen py-20 px-4">
      <Card className="p-10 space-y-4 h-fit">
        <h1 className="text-4xl">Application form</h1>
        <Separator />
        <RenderMemberData member={currentMember} />
        <Separator />
        <div>
          Confirm that the information above is correct before submitting the
          application. If not, please update your information in the profile
          page.
        </div>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="role_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Membership type</FormLabel>
                  <Select
                    onValueChange={(value) => {
                      const targetableRole = targetableRoles?.find(
                        (role) => role.role_name === value
                      );
                      const validUntil = targetableRole?.valid_until;
                      const paymentLink = targetableRole?.payment_link;

                      if (!validUntil) {
                        throw new Error(
                          `Role ${value} not found in targetable roles`
                        );
                      }

                      setPaymentLink(paymentLink ?? null)
                      form.setValue("valid_until", validUntil);
                      return field.onChange(value);
                    }}
                  >
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue placeholder="Select role" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      {targetableRoles?.map((role) => (
                        <SelectItem value={role.role_name} key={role.role_name+role.valid_until}>
                          {role.role_name}{" "}
                          <span>
                            (Valid until {role.valid_until.toLocaleDateString()}
                            )
                          </span>
                        </SelectItem>
                      ))}
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
                  <FormLabel>Application text</FormLabel>
                  <FormControl>
                    <Textarea placeholder="Short application text" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <Button type="submit">{
              paymentLink ? (
                "Proceed to payment"
              ) : (
                "Submit application"
              )
              }</Button>
          </form>
        </Form>
      </Card>
    </main>
  );
};

export default ApplicationForm;
