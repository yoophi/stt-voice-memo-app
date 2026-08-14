import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui/card";

type FoundationStatusProps = {
  productName: string;
};

export function FoundationStatus({ productName }: FoundationStatusProps) {
  return (
    <Card className="w-full max-w-md">
      <CardHeader>
        <CardTitle>
          <h1>{productName}</h1>
        </CardTitle>
        <CardDescription>모바일 음성 메모를 위한 앱 기반이 준비되었습니다.</CardDescription>
      </CardHeader>
      <CardContent>
        <p className="text-muted-foreground">녹음과 음성 변환 기능은 다음 단계에서 제공됩니다.</p>
      </CardContent>
    </Card>
  );
}
