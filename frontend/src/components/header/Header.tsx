import {
  NavigationMenu,
  NavigationMenuList,
  NavigationMenuItem,
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
import { ExternalLinkIcon, PersonIcon } from "@radix-ui/react-icons";
import { Logout } from "./Logout";
import { useGetPublicConfig } from "../../lib/api";

const Header = () => {
  const { data: config } = useGetPublicConfig();
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
          <NavigationMenuItem>
            <NavigationMenuLink
              asChild
              className={navigationMenuTriggerStyle()}
            >
              <Link to="/role-groups">Groups</Link>
            </NavigationMenuLink>
          </NavigationMenuItem>
          <NavigationMenuItem>
            <NavigationMenuLink
              asChild
              className={navigationMenuTriggerStyle()}
            >
              <Link to="/attributes">Attributes</Link>
            </NavigationMenuLink>
          </NavigationMenuItem>
          <NavigationMenuLink asChild className={navigationMenuTriggerStyle()}>
            <Link to="/logs">Logs</Link>
          </NavigationMenuLink>
          <NavigationMenuLink asChild className={navigationMenuTriggerStyle()}>
            <Link to="/email-templates">Templates</Link>
          </NavigationMenuLink>
          <NavigationMenuLink asChild className={navigationMenuTriggerStyle()}>
            <Link to="/marketing-tags">Marketing tags</Link>
          </NavigationMenuLink>
          <NavigationMenuLink asChild className={navigationMenuTriggerStyle()}>
            <Link to="/data">Data</Link>
          </NavigationMenuLink>
          {config?.keycloak_admin_url && (
            <NavigationMenuLink
              asChild
              className={navigationMenuTriggerStyle()}
            >
              <a
                href={config.keycloak_admin_url}
                target="_blank"
                rel="noopener noreferrer"
              >
                Keycloak <ExternalLinkIcon className="ml-1 inline h-3 w-3" />
              </a>
            </NavigationMenuLink>
          )}
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
