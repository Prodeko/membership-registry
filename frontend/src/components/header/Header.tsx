import {
  NavigationMenu,
  NavigationMenuList,
  NavigationMenuItem,
  NavigationMenuTrigger,
  NavigationMenuContent,
  NavigationMenuLink,
} from "@radix-ui/react-navigation-menu";
import { Separator } from "../ui/separator";
import { Link } from "react-router-dom";
import { navigationMenuTriggerStyle } from "../ui/navigation-menu";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "../ui/dropdown-menu";
import { PersonIcon } from "@radix-ui/react-icons";
import { Button } from "../ui/button";

const Header = () => {
  return (
    <NavigationMenu>
      <div className="flex justify-between">
        <NavigationMenuList className="flex space-x-8 px-6 py-4">
            <Link to="/">
              <img src="/prodeko.svg" alt="Prodeko" className="h-10" />
            </Link>
          <NavigationMenuItem>
            <Link to="/members">
              <NavigationMenuLink className={navigationMenuTriggerStyle()}>
                Member list
              </NavigationMenuLink>
            </Link>
          </NavigationMenuItem>
          <NavigationMenuItem>
            <Link to="/applications">
              <NavigationMenuLink className={navigationMenuTriggerStyle()}>
                Applications
              </NavigationMenuLink>
            </Link>
          </NavigationMenuItem>
          <NavigationMenuItem>
            <Link to="/roles">
              <NavigationMenuLink className={navigationMenuTriggerStyle()}>
                Roles
              </NavigationMenuLink>
            </Link>
          </NavigationMenuItem>
          <Link to="/logs">
            <NavigationMenuLink className={navigationMenuTriggerStyle()}>
              Logs
            </NavigationMenuLink>
          </Link>
        </NavigationMenuList>
        <DropdownMenu>
          <DropdownMenuTrigger className="p-4">
            <PersonIcon className="h-6 w-6" />
          </DropdownMenuTrigger>
          <DropdownMenuContent >
            <DropdownMenuItem>
              <Button variant="ghost" onClick={() => console.log("Log out")}>
                Log out
              </Button>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <Separator />
    </NavigationMenu>
  );
};

export default Header;
