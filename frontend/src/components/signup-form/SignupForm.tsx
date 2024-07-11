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
import { Input } from "@/components/ui/input";
import { COUNTRIES, FINNISH_MUNICIPALITIES } from "@/lib/constants";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { Card } from "../ui/card";
import { Checkbox } from "../ui/checkbox";
import MunicipalitySelect from "./MunicipalitySelect";
import { useCreateMember } from "@/lib/api";
import { useSearchParams } from "react-router-dom";

const formSchema = z.object({
  first_name: z.string({ required_error: "First name is required" }).min(1),
  last_name: z.string({ required_error: "Last name is required" }).min(1),
  email: z.string({ required_error: "Email is required" }).email(),
  home_municipality: z.enum([...FINNISH_MUNICIPALITIES, ...COUNTRIES]),
  has_accepted_policies: z.boolean({
    required_error: "You must accept the policies",
  }),
});

export type SingupFormValues = z.infer<typeof formSchema>;

const SignupForm = () => {
  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      first_name: "",
      last_name: "",
      email: "",
      home_municipality: "Espoo",
      has_accepted_policies: false,
    },
  });

  const [searchParams] = useSearchParams();

  const { mutate:  createMember } = useCreateMember();

  const onSubmit = (values: z.infer<typeof formSchema>) => {
    createMember(values, {
      onSuccess: (data) => {
        // TODO: Remove this when proper authentication is implemented
        window.localStorage.setItem("user_id", data.user_id);
        window.location.href = searchParams.get("redirect") || "/";
      },
      onError: (error) => {
        // TODO: Handle error
        alert(error.message);
      },
    });
  };

  return (
    <main className="flex justify-center align-middle h-screen w-screen py-20 px-4">
      <Card className="p-10 space-y-4 h-fit">
        <h1 className="text-4xl">Signup form</h1>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="first_name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>First name</FormLabel>
                  <FormControl>
                    <Input placeholder="First name" {...field} />
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
                    <Input placeholder="Last name" {...field} />
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
                  <FormLabel>Email</FormLabel>
                  <FormControl>
                    <Input placeholder="Email" {...field} />
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
              name="has_accepted_policies"
              render={({ field }) => (
                <FormItem className="flex flex-row items-start space-x-3 space-y-0 ">
                  <FormLabel>
                    I have read and accept the <a href="#">policies</a>
                  </FormLabel>
                  <FormControl>
                    <Checkbox
                      checked={field.value}
                      onCheckedChange={field.onChange}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <Button type="submit">Submit</Button>
          </form>
        </Form>
      </Card>
    </main>
  );
};

export default SignupForm;
