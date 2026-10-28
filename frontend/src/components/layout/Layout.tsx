import { ReactNode } from "react";
import Header from "../header/Header";

const Layout = ({ children }: { children: ReactNode }) => {
  return (
    <div className="flex flex-col h-screen">
      <Header />
      <main className="min-w-0 p-4 md:p-6">{children}</main>
    </div>
  );
};

export default Layout;
