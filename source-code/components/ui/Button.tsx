import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
    size?: 'sm' | 'md' | 'lg' | 'xl';
    active?: boolean;
}

export const Button: React.FC<ButtonProps> = ({
    children,
    variant = 'secondary',
    size = 'md',
    active = false,
    className = '',
    ...props
}) => {
    const baseStyle = "font-medium transition-all duration-200 rounded-sm focus:outline-none flex items-center justify-center gap-2 relative overflow-hidden";

    const variants = {
        primary: `bg-steam-blue text-white hover:bg-steam-blueHover shadow-[0_0_15px_rgba(26,159,255,0.3)] border border-transparent`,
        secondary: `bg-[#2a3038] text-steam-text hover:bg-[#3d4450] hover:text-white border border-gray-700`,
        danger: `bg-red-900/40 text-red-200 hover:bg-red-600 hover:text-white border border-red-900`,
        ghost: `bg-transparent text-steam-text hover:bg-white/10`
    };

    const sizes = {
        sm: "px-3 py-1.5 text-sm",
        md: "px-6 py-3 text-base",
        lg: "px-8 py-4 text-lg font-bold",
        xl: "px-10 py-6 text-xl font-bold"
    };

    const activeStyle = active
    ? "ring-2 ring-white/80 bg-white text-black scale-[1.02]"
    : "";

    return (
        <button
        className={`${baseStyle} ${variants[variant]} ${sizes[size]} ${activeStyle} ${className}`}
        {...props}
        >
        {children}
        </button>
    );
};
