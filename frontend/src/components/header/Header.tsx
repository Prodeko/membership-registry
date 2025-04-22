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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import { PersonIcon } from "@radix-ui/react-icons";
import { Button } from "../ui/button";
import { Logout } from "./Logout";

const Header = () => {
  return (
    <NavigationMenu>
      <div className="flex justify-between">
        <NavigationMenuList className="flex space-x-8 px-6 py-4">
          <Link to="/">
            <img src="/prodeko.svg" alt="Prodeko" className="h-10" />
          </Link>
          <NavigationMenuItem>
            <NavigationMenuLink
              asChild
              className={navigationMenuTriggerStyle()}
            >
              <Link to="/members">Member list</Link>
            </NavigationMenuLink>
          </NavigationMenuItem>
          <NavigationMenuItem>
            <NavigationMenuLink
              asChild
              className={navigationMenuTriggerStyle()}
            >
              <Link to="/applications">Applications</Link>
            </NavigationMenuLink>
          </NavigationMenuItem>
          <NavigationMenuItem>
            <NavigationMenuLink
              asChild
              className={navigationMenuTriggerStyle()}
            >
              <Link to="/roles">Roles</Link>
            </NavigationMenuLink>
          </NavigationMenuItem>
          <NavigationMenuLink asChild className={navigationMenuTriggerStyle()}>
            <Link to="/logs">Logs</Link>
          </NavigationMenuLink>
        </NavigationMenuList>
        <DropdownMenu>
          <DropdownMenuTrigger className="p-4">
            <PersonIcon className="h-6 w-6" />
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuItem>
              <Logout />
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <Separator />
    </NavigationMenu>
  );
};

export default Header;
