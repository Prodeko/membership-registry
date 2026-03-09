import { Link } from "react-router-dom";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { Separator } from "../ui/separator";

const Success = () => {
  return (
    <main className="flex justify-center items-center min-h-screen w-screen px-4">
      <Card className="max-w-md w-full">
        <CardHeader className="text-center">
          <div className="flex justify-center mb-4">
            <img src="/prodeko.svg" alt="Prodeko" className="h-20" />
          </div>
          <CardTitle className="text-2xl">Application submitted</CardTitle>
        </CardHeader>
        <Separator className="mx-6" />
        <CardContent className="pt-6 text-center space-y-2">
          <p className="text-muted-foreground">
            Your application has been received and is now being reviewed. You
            will be notified by email once a decision has been made.
          </p>
        </CardContent>
        <CardFooter className="flex flex-col gap-2">
          <Button asChild className="w-full">
            <Link to="/home">Go to home page</Link>
          </Button>
          <Button asChild variant="outline" className="w-full">
            <Link to="/apply">Submit another application</Link>
          </Button>
        </CardFooter>
      </Card>
    </main>
  );
};

export default Success;
