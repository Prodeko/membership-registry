import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { InfoCircledIcon } from "@radix-ui/react-icons";

interface InfoTooltipProps {
  children: React.ReactNode;
}

const InfoTooltip: React.FC<InfoTooltipProps> = ({ children }) => {
  return (
    <Tooltip>
      <TooltipTrigger>
        <InfoCircledIcon className="inline-block ml-2 text-muted-foreground" />
      </TooltipTrigger>
      <TooltipContent>{children}</TooltipContent>
    </Tooltip>
  );
};

export default InfoTooltip;
