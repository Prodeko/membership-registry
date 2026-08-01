import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { Separator } from "../ui/separator";

const PaymentSuccess = () => {
  const { t } = useTranslation();

  return (
    <main className="flex justify-center items-center min-h-screen w-screen px-4">
      <Card className="max-w-md w-full">
        <CardHeader className="text-center">
          <div className="flex justify-center mb-4">
            <img src="/prodeko.svg" alt="Prodeko" className="h-20" />
          </div>
          <CardTitle className="text-2xl">
            {t("payment_success.title")}
          </CardTitle>
        </CardHeader>
        <Separator className="mx-6" />
        <CardContent className="pt-6 text-center space-y-2">
          <p className="text-muted-foreground">
            {t("payment_success.message")}
          </p>
        </CardContent>
        <CardFooter>
          <Button asChild className="w-full">
            <Link to="/home">{t("payment_success.go_home")}</Link>
          </Button>
        </CardFooter>
      </Card>
    </main>
  );
};

export default PaymentSuccess;
