import * as React from 'react';
import { FolderIcon, FolderOpenIcon } from 'lucide-react';

import {
  Files as FilesPrimitive,
  FilesHighlight as FilesHighlightPrimitive,
  FolderItem as FolderItemPrimitive,
  FolderHeader as FolderHeaderPrimitive,
  FolderTrigger as FolderTriggerPrimitive,
  Folder as FolderPrimitive,
  FolderIcon as FolderIconPrimitive,
  FileLabel as FileLabelPrimitive,
  FolderContent as FolderContentPrimitive,
  File as FilePrimitive,
  type FilesProps as FilesPrimitiveProps,
  type FolderItemProps as FolderItemPrimitiveProps,
  type FolderContentProps as FolderContentPrimitiveProps,
  type FileProps as FilePrimitiveProps,
  type FileLabelProps as FileLabelPrimitiveProps,
  FileHighlight,
  FolderHighlight,
} from '@/components/animate-ui/primitives/radix/files';
import { cn } from '@/lib/utils';
import { useRef } from 'react';
import { FileCogIcon, FileCogIconHandle } from '@/components/ui/file-cog';

type GitStatus = 'untracked' | 'modified' | 'deleted';

type FilesProps = FilesPrimitiveProps;

function Files({ className, children, ...props }: FilesProps) {
  return (
    <FilesPrimitive className={cn('p-2 w-full', className)} {...props}>
      <FilesHighlightPrimitive className="rounded-lg pointer-events-none">
        {children}
      </FilesHighlightPrimitive>
    </FilesPrimitive>
  );
}

type SubFilesProps = FilesProps;

function SubFiles(props: SubFilesProps) {
  return <FilesPrimitive {...props} />;
}

type FolderItemProps = FolderItemPrimitiveProps;

function FolderItem(props: FolderItemProps) {
  return <FolderItemPrimitive {...props} />;
}

type FolderTriggerProps = FileLabelPrimitiveProps & {
  gitStatus?: GitStatus;
};

function FolderTrigger({
  children,
  className,
  gitStatus,
  ...props
}: FolderTriggerProps) {
  return (
    <FolderHeaderPrimitive>
      <FolderTriggerPrimitive className="w-full text-start group">
        <FolderHighlight asChild>
          <FolderPrimitive
            className={cn(
              "flex items-center justify-between gap-2 p-2 rounded-xl transition-colors relative",
              "bg-transparent hover:text-[#1a0b2e]",
              className
            )}
          >
            <div className="flex items-center gap-2 relative z-10">
              <FolderIconPrimitive
                closeIcon={<FolderIcon className="size-4.5" />}
                openIcon={<FolderOpenIcon className="size-4.5" />}
              />
              <FileLabelPrimitive className="text-sm" {...props}>
                {children}
              </FileLabelPrimitive>
            </div>

            {gitStatus && (
              <span
                className={cn(
                  'rounded-full size-2 relative z-10',
                  gitStatus === 'untracked' && 'bg-green-400',
                  gitStatus === 'modified' && 'bg-amber-400',
                  gitStatus === 'deleted' && 'bg-red-400',
                )}
              />
            )}
          </FolderPrimitive>
        </FolderHighlight>
      </FolderTriggerPrimitive>
    </FolderHeaderPrimitive>
  );
}

type FolderContentProps = FolderContentPrimitiveProps;

function FolderContent(props: FolderContentProps) {
  return (
    <div className="relative ml-6 before:absolute before:-left-2 before:inset-y-0 before:w-px before:h-full before:bg-border">
      <FolderContentPrimitive {...props} />
    </div>
  );
}

type FileItemProps = FilePrimitiveProps & {
  icon?: React.ElementType;
  gitStatus?: GitStatus;
  value?: string;
};

function FileItem({ children, className, value, onClick, ...props }: FileItemProps) {
  const cogRef = useRef<FileCogIconHandle>(null);
  const fileName = children?.toString() || "";

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    cogRef.current?.startAnimation();
    if (onClick) onClick(e);
    setTimeout(() => {
      cogRef.current?.stopAnimation();
    }, 400);
  };

  return (
    <FileHighlight asChild value={value ?? fileName}>
      <FilePrimitive
        className={cn(
          "p-2 w-full rounded-xl transition-all relative cursor-pointer",
          "bg-transparent hover:bg-[#d8b4fe] hover:text-[#1a0b2e]",
          className
        )}
        onClick={handleClick}
        {...props}
      >
        <div className="flex items-center relative z-10 w-full overflow-visible">
          <div className="shrink-0 flex items-center justify-center size-5 mr-3 -translate-y-0.75">
            <FileCogIcon
              ref={cogRef}
              style={{
                strokeWidth: '1.6px',
                transform: 'scale(0.7)'
              }}
              className="size-5 transition-colors group-hover:text-[#1a0b2e]"
            />
          </div>

          <FileLabelPrimitive className="text-[13px] font-medium whitespace-nowrap overflow-visible leading-none">
            {children}
          </FileLabelPrimitive>
        </div>
      </FilePrimitive>
    </FileHighlight>
  );
}

export {
  Files,
  FolderItem,
  FolderTrigger,
  FolderContent,
  FileItem,
  SubFiles,
  type FilesProps,
  type FolderItemProps,
  type FolderTriggerProps,
  type FolderContentProps,
  type FileItemProps,
  type SubFilesProps,
};
